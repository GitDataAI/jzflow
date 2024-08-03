use anyhow::{anyhow, Result};
use compute_unit_runner::fs_cache::FileCache;
use jz_action::core::models::{DataRecord, DataState, DbRepo, Direction, TrackerState};
use jz_action::network::datatransfer::MediaDataBatchResponse;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{self, Instant};
use tokio::select;
use tracing::{debug, error, info, warn};

pub struct ChannelTracker<R>
where
    R: DbRepo,
{
    pub(crate) repo: R,

    pub(crate) name: String,

    pub(crate) buf_size: usize,

    pub(crate) fs_cache: Arc<dyn FileCache>,

    pub(crate) local_state: TrackerState,

    pub(crate) upstreams: Vec<String>,

    pub(crate) downstreams: Vec<String>,

    pub(crate) incoming_tx: Option<Sender<(MediaDataBatchResponse, oneshot::Sender<Result<()>>)>>,

    pub(crate) request_tx:
        Option<Sender<((), oneshot::Sender<Result<Option<MediaDataBatchResponse>>>)>>,
}

impl<R> ChannelTracker<R>
where
    R: DbRepo,
{
    pub(crate) fn new(repo: R, fs_cache: Arc<dyn FileCache>, name: &str, buf_size: usize) -> Self {
        ChannelTracker {
            name: name.to_string(),
            repo: repo,
            fs_cache,
            buf_size,
            local_state: TrackerState::Init,
            upstreams: vec![],
            downstreams: vec![],
            request_tx: None,
            incoming_tx: None,
        }
    }

    pub(crate) async fn route_data(&mut self) -> Result<()> {
        if self.upstreams.len() == 0 {
            return Err(anyhow!("no upstream"));
        }

        let (incoming_tx, mut incoming_rx) = mpsc::channel(1);
        self.incoming_tx = Some(incoming_tx);

        let (request_tx, mut request_rx) = mpsc::channel(1);
        self.request_tx = Some(request_tx);

        {
            //process clean data
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let fs_cache = self.fs_cache.clone();
            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(30));
                loop {
                    let now = Instant::now();
                    info!("gc start");
                    tokio::select! {
                        _ = interval.tick() => {
                            match db_repo.list_by_node_name_and_state(&node_name, DataState::EndRecieved).await {
                                Ok(datas) => {
                                    for data in datas {
                                        match fs_cache.remove(&data.id).await {
                                            Ok(_)=>{
                                                if let Err(err) =  db_repo.update_state(&node_name, &data.id, Direction::In, DataState::Clean, None).await{
                                                    error!("mark data as client receive {err}");
                                                    continue;
                                                }
                                                debug!("remove data {}", &data.id);
                                            },
                                            Err(err)=> error!("remove data {err}")
                                        }
                                    }
                                },
                                Err(err) => error!("list datas fail {err}"),
                            }
                        }
                    }
                    info!("gc end {:?}", now.elapsed());
                }
            });
        }
        {
            //process request data
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let fs_cache = self.fs_cache.clone();

            tokio::spawn(async move {
                loop {
                    select! {
                     Some((_, resp)) = request_rx.recv() => { //make this params
                        match db_repo.find_data_and_mark_state(&node_name, Direction::In, DataState::SelectForSend).await {
                            std::result::Result::Ok(Some(record)) =>{
                                info!("return downstream's datarequest and start response data {}", &record.id);
                                match fs_cache.read(&record.id).await {
                                    Ok(databatch)=>{
                                        //response this data's position
                                        resp.send(Ok(Some(databatch))).expect("channel only read once");
                                    },
                                    Err(err)=>{
                                        error!("read files {} {err}", record.id);
                                        resp.send(Err(err)).expect("channel only read once");
                                    }
                                }

                            },
                            std::result::Result::Ok(None)=>{
                                resp.send(Ok(None)).expect("channel only read once");
                            }
                            Err(err)=>{
                                error!("insert  incoming data record to mongo {err}");
                                //TODO send error message?
                                resp.send(Err(err)).expect("channel only read once");
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
            let fs_cache = self.fs_cache.clone();
            tokio::spawn(async move {
                loop {
                    select! {
                     Some((data_batch, resp)) = incoming_rx.recv() => { //make this params
                        //save to fs
                        //create input directory
                        let now = Instant::now();
                        let id = data_batch.id.clone();
                        let size = data_batch.size;
                        //check limit
                       if let Err(err) =  db_repo.count_pending(&node_name,Direction::In).await.and_then(|count|{
                            if count > buf_size {
                                Err(anyhow!("has reach limit current:{count} limit:{buf_size}"))
                            } else {
                                Ok(())
                            }
                        }){
                            resp.send(Err(anyhow!("cannt query limit from mongo {err}"))).expect("request alread listen this channel");
                            continue;
                        }

                        // processed before
                        match db_repo.find_by_node_id(&node_name,&id,Direction::In).await  {
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

                        // code below this can be move another coroutine

                        //write batch files
                        if let Err(err) = fs_cache.write(data_batch).await {
                            error!("write files to disk fail {}", err);
                            resp.send(Err(err.into())).expect("request alread listen this channel");
                            continue;
                        }

                        //insert record to database
                        if let Err(err) = db_repo.insert_new_path(&DataRecord{
                            node_name: node_name.clone(),
                            id:id.clone(),
                            size: size,
                            state: DataState::Received,
                            sent: vec![],
                            direction:Direction::In,
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

        Ok(())
    }

    pub async fn apply_db_state(
        repo: R,
        name: &str,
        program: Arc<Mutex<ChannelTracker<R>>>,
    ) -> Result<()> {
        let mut interval = time::interval(time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let record = repo
                .get_node_by_name(name)
                .await
                .expect("record has inserted in controller or network error");
            debug!("{} fetch state from db", record.node_name);
            match record.state {
                TrackerState::Ready => {
                    let mut program_guard = program.lock().await;

                    if matches!(program_guard.local_state, TrackerState::Init) {
                        info!("set to ready state {:?}", record.upstreams);
                        //start
                        program_guard.local_state = TrackerState::Ready;
                        program_guard.upstreams = record.upstreams;
                        program_guard.downstreams = record.downstreams;
                        program_guard.route_data().await?;
                    }
                }
                TrackerState::Stop => {
                    todo!()
                }
                _ => {}
            }
        }
    }
}
