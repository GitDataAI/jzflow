use anyhow::{
    anyhow,
    Result,
};
use chrono::Utc;
use futures::future::try_join_all;
use jiaoziflow::{
    core::db::{
        DataRecord,
        DataState,
        Direction,
        JobDbRepo,
        TrackerState,
    },
    network::datatransfer::MediaDataBatchResponse,
};
use nodes_sdk::{
    fs_cache::FileCache,
    metadata::is_metadata,
    MessageSender,
};
use std::{
    sync::Arc,
    time::Duration,
};
use tokio::{
    select,
    sync::mpsc::{
        self,
    },
    task::JoinSet,
    time::{
        self,
        Instant,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    error,
    info,
    warn,
};

pub struct ChannelTracker<R>
where
    R: JobDbRepo,
{
    pub(crate) repo: R,

    pub(crate) name: String,

    pub(crate) buf_size: usize,

    pub(crate) data_cache: Arc<dyn FileCache>,

    pub(crate) local_state: TrackerState,

    pub(crate) up_nodes: Vec<String>,

    pub(crate) upstreams: Vec<String>,

    pub(crate) incoming_tx: Option<MessageSender<MediaDataBatchResponse, ()>>,

    pub(crate) request_tx: Option<MessageSender<(), Option<MediaDataBatchResponse>>>,
}

impl<R> ChannelTracker<R>
where
    R: JobDbRepo,
{
    pub(crate) fn new(
        repo: R,
        data_cache: Arc<dyn FileCache>,
        name: &str,
        buf_size: usize,
        up_nodes: Vec<String>,
        upstreams: Vec<String>,
    ) -> Self {
        ChannelTracker {
            name: name.to_string(),
            repo,
            data_cache,
            buf_size,
            local_state: TrackerState::Init,
            up_nodes,
            upstreams,
            request_tx: None,
            incoming_tx: None,
        }
    }

    pub fn run_backend(
        &mut self,
        join_set: &mut JoinSet<Result<()>>,
        token: CancellationToken,
    ) -> Result<()> {
        //process backent task to do revert clean data etc
        let db_repo = self.repo.clone();
        let node_name = self.name.clone();
        let data_cache = self.data_cache.clone();
        let token = token.clone();
        let up_nodes = self.up_nodes.clone();

        join_set.spawn(async move {
            let mut interval = time::interval(Duration::from_secs(30));
            loop {
                let now = Instant::now();
                info!("backend thread start");
                select! {
                    _ = token.cancelled() => {
                        return Ok(());
                     }
                    _ = interval.tick() => {
                        if let Err(err) = {
                            // clean data
                            match db_repo.list_by_node_name_and_state(&node_name, &DataState::EndRecieved).await {
                                Ok(datas) => {
                                    for data in datas {
                                        if data.is_metadata {
                                            if let Err(err) =  db_repo.update_state(&node_name, &data.id, &Direction::In, &DataState::KeeptForMetadata, None).await{
                                                error!("mark metadata data as client receive {err}");
                                                continue;
                                            }
                                            info!("mark metadata as cleint received")
                                        }else {
                                            match data_cache.remove(&data.id).await {
                                                Ok(_)=>{
                                                    if let Err(err) =  db_repo.update_state(&node_name, &data.id, &Direction::In, &DataState::Clean, None).await{
                                                        error!("mark data as client receive {err}");
                                                        continue;
                                                    }
                                                    debug!("remove data {}", &data.id);
                                                },
                                                Err(err)=> error!("remove data {err}")
                                            }
                                        }
                                    }
                                },
                                Err(err) => error!("list datas fail {err}"),
                            }
                            //select for sent
                            db_repo.revert_no_success_sent(&node_name, &Direction::Out).await
                            .map_err(|err|anyhow!("revert data {err}"))
                            .map(|count| info!("revert {count} SelectForSent data to Received"))?;

                            //check ready if both upnodes is finish and no pending data, we think it finish
                            let is_all_success = try_join_all(up_nodes.iter().map(|node_name|db_repo.get_node_by_name(node_name))).await
                            .map_err(|err|anyhow!("query node data {err}"))?
                            .iter()
                            .all(|node| matches!(node.state, TrackerState::Finish));

                            if is_all_success {
                                let running_state = &[
                                    &DataState::Received,
                                    &DataState::Assigned,
                                    &DataState::EndRecieved
                                ];
                                if db_repo.count(&node_name, running_state.as_slice(), Some(&Direction::Out)).await? == 0 {
                                    db_repo.update_node_by_name(&node_name, TrackerState::Finish).await.map_err(|err|anyhow!("update node data {err}"))?;
                                    info!("node was finished, not need to run backend");
                                    return    anyhow::Ok(());
                                }
                            }
                            info!("backend thread end {:?}", now.elapsed());
                            anyhow::Ok(())
                        }{
                            error!("error in run backend {err}");
                        }
                    }
                }
                info!("backend thread end {:?}", now.elapsed());
            }
        });
        Ok(())
    }

    pub(crate) async fn route_data(
        &mut self,
        token: CancellationToken,
    ) -> Result<JoinSet<Result<()>>> {
        if self.upstreams.is_empty() {
            return Err(anyhow!("no upstream"));
        }

        let (incoming_tx, mut incoming_rx) = mpsc::channel(1);
        self.incoming_tx = Some(incoming_tx);

        let (request_tx, mut request_rx) = mpsc::channel(1);
        self.request_tx = Some(request_tx);

        let mut join_set: JoinSet<Result<()>> = JoinSet::new();
        {
            //process request data
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let data_cache = self.data_cache.clone();
            let token = token.clone();

            join_set.spawn(async move {
                loop {
                    select! {
                        _ = token.cancelled() => {
                            return Ok(());
                         }
                     Some((_, resp)) = request_rx.recv() => { //make this params
                        loop {
                            match db_repo.find_data_and_mark_state(&node_name, &Direction::In, &DataState::SelectForSend).await {
                                std::result::Result::Ok(Some(record)) =>{
                                    info!("return downstream's datarequest and start response data {}", &record.id);
                                    match data_cache.read(&record.id).await {
                                        Ok(databatch)=>{
                                            //response this data's position
                                            resp.send(Ok(Some(databatch))).expect("channel only read once");
                                            break;
                                        },
                                        Err(err)=>{
                                            error!("read files {} {err}, try to find another data", record.id);
                                            if let Err(err) = db_repo.update_state(&node_name,&record.id, &Direction::In, &DataState::Error, None).await {
                                                error!("mark data {} to error {err}", &record.id);
                                            }
                                            continue;
                                        }
                                    }
                                },
                                std::result::Result::Ok(None)=>{
                                    resp.send(Ok(None)).expect("channel only read once");
                                    break;
                                }
                                Err(err)=>{
                                    error!("insert  incoming data record to mongo {err}");
                                    //TODO send error message?
                                    resp.send(Err(err)).expect("channel only read once");
                                    break;
                                }
                            }
                        }
                     },
                    }
                }
            });
        }

        {
            //process incoming data
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let buf_size = self.buf_size;
            let data_cache = self.data_cache.clone();
            let token = token.clone();

            join_set.spawn(async move {
                loop {
                    select!{
                        _ = token.cancelled() => {
                            return Ok(());
                         }
                     Some((data_batch, resp)) = incoming_rx.recv() => { //make this params
                        //save to fs
                        //create input directory
                        let now = Instant::now();
                        let id = data_batch.id.clone();
                        let size = data_batch.size;

                        // processed before
                        match db_repo.find_by_node_id(&node_name,&id, &Direction::In).await  {
                            Ok(Some(_))=>{
                                warn!("data {} processed before", &id);
                                resp.send(Ok(())).expect("request alread listen this channel");
                                continue;
                            }
                            Err(err)=>{
                                error!("query mongo by id {err}");
                                resp.send(Err(err)).expect("request alread listen this channel");
                                continue;
                            }
                            _=>{}
                        }
                        let is_data_metadata = if is_metadata(&id) {
                            //check limit for plain data
                            if let Err(err) =  db_repo.count(&node_name,&[&DataState::Received], Some(&Direction::In)).await.and_then(|count|{
                                if count > buf_size {
                                    Err(anyhow!("has reach limit current:{count} limit:{buf_size}"))
                                } else {
                                    Ok(())
                                }
                            }){
                                resp.send(Err(anyhow!("cannt query limit from mongo {err}"))).expect("request alread listen this channel");
                                continue;
                            }
                            true
                        }else {
                            false
                        };

                        //write batch files
                        if let Err(err) = data_cache.write(data_batch).await {
                            error!("write files to disk fail {}", err);
                            resp.send(Err(err)).expect("request alread listen this channel");
                            continue;
                        }

                        //insert record to database
                        let tm =Utc::now().timestamp();
                        if let Err(err) = db_repo.insert_new_path(&DataRecord{
                            node_name: node_name.clone(),
                            id:id.clone(),
                            size,
                            is_metadata:is_data_metadata,
                            state: DataState::Received,
                            sent: vec![],
                            direction:Direction::In,
                            created_at:tm,
                            updated_at:tm,
                        }).await{
                            error!("insert a databatch {err}");
                            resp.send(Err(err)).expect("request alread listen this channel");
                            continue;
                        }

                        resp.send(Ok(())).expect("request alread listen this channel ");
                        info!("insert a data batch in {} {:?}", &id, now.elapsed());
                     },
                    }
                }
            });
        }

        Ok(join_set)
    }
}
