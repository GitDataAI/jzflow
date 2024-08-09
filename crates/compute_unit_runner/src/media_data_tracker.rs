use crate::ipc::{
    AvaiableDataResponse,
    CompleteDataReq,
    SubmitOuputDataReq,
};
use anyhow::{
    anyhow,
    Result,
};

use chrono::Utc;
use jz_action::{
    core::db::{
        DataRecord,
        DataState,
        Direction,
        JobDbRepo,
        TrackerState,
    },
    network::{
        common::Empty,
        datatransfer::data_stream_client::DataStreamClient,
    },
};
use std::{
    sync::Arc,
    time::Duration,
    vec,
};

use tonic::{
    transport::Channel,
    Code,
};
use tracing::{
    debug,
    error,
    info,
    warn,
};

use nodes_sdk::{
    fs_cache::FileCache,
    multi_sender::MultiSender,
    MessageSender,
};
use tokio::{
    select,
    sync::{
        broadcast,
        mpsc,
        RwLock,
    },
    task::JoinSet,
    time::{
        self,
        sleep,
        Instant,
    },
};

use futures::future::try_join_all;
use tokio_util::sync::CancellationToken;

pub struct MediaDataTracker<R>
where
    R: JobDbRepo,
{
    pub(crate) name: String,

    pub(crate) buf_size: usize,

    pub(crate) fs_cache: Arc<dyn FileCache>,

    pub(crate) repo: R,

    pub(crate) local_state: Arc<RwLock<TrackerState>>,

    pub(crate) up_nodes: Vec<String>,

    pub(crate) upstreams: Vec<String>,

    pub(crate) downstreams: Vec<String>,

    // channel for process avaiable data request
    pub(crate) ipc_process_data_req_tx: Option<MessageSender<(), Option<AvaiableDataResponse>>>,

    // channel for response complete data. do clean work when receive this request
    pub(crate) ipc_process_completed_data_tx: Option<MessageSender<CompleteDataReq, ()>>,

    // channel for submit output data
    pub(crate) ipc_process_submit_output_tx: Option<MessageSender<SubmitOuputDataReq, ()>>,

    // channel for receive finish state from user container
    pub(crate) ipc_process_finish_state_tx: Option<MessageSender<(), ()>>,
}
impl<R> MediaDataTracker<R>
where
    R: JobDbRepo,
{
    pub fn new(
        repo: R,
        name: &str,
        fs_cache: Arc<dyn FileCache>,
        buf_size: usize,
        up_nodes: Vec<String>,
        upstreams: Vec<String>,
        downstreams: Vec<String>,
    ) -> Self {
        MediaDataTracker {
            fs_cache,
            buf_size,
            name: name.to_string(),
            repo,
            local_state: Arc::new(RwLock::new(TrackerState::Init)),
            up_nodes,
            upstreams,
            downstreams,
            ipc_process_submit_output_tx: None,
            ipc_process_completed_data_tx: None,
            ipc_process_data_req_tx: None,
            ipc_process_finish_state_tx: None,
        }
    }
}

impl<R> MediaDataTracker<R>
where
    R: JobDbRepo,
{
    pub fn run_backend(
        &mut self,
        join_set: &mut JoinSet<Result<()>>,
        token: CancellationToken,
    ) -> Result<()> {
        //process backent task to do revert clean data etc
        let db_repo = self.repo.clone();
        let up_nodes = self.up_nodes.clone();
        let node_name = self.name.clone();
        join_set.spawn(async move {
                let mut interval = time::interval(Duration::from_secs(30));
                loop {
                    select! {
                        _ = token.cancelled() => {
                           return Ok(());
                        }
                        _ = interval.tick() => {
                         if let Err(err) = {
                            let now = Instant::now();
                            info!("backend thread start");
                            //select for sent
                            db_repo.revert_no_success_sent(&node_name, &Direction::Out).await
                            .map_err(|err|anyhow!("revert data {err}"))
                            .map(|count| info!("revert {count} SelectForSent data to Received"))?;

                            //check ready if both upnodes is finish and no pending data, we think it finish
                            if !up_nodes.is_empty() {
                                let is_all_success = try_join_all(up_nodes.iter().map(|node_name|db_repo.get_node_by_name(node_name))).await
                                .map_err(|err|anyhow!("query node data {err}"))?
                                .iter()
                                .any(|node| matches!(node.state, TrackerState::Finish));

                                if is_all_success {
                                    let running_state = &[
                                            &DataState::Received,
                                            &DataState::Assigned,
                                            &DataState::SelectForSend,
                                            &DataState::PartialSent
                                    ];

                                    if db_repo.count(&node_name, running_state.as_slice(), &Direction::Out).await? == 0 &&
                                    db_repo.count(&node_name,  &[&DataState::Received,&DataState::Assigned], &Direction::In).await? == 0 {
                                        db_repo.update_node_by_name(&node_name, TrackerState::Finish).await.map_err(|err|anyhow!("update node data {err}"))?;
                                        info!("node was finished, not need to run backend");
                                        return anyhow::Ok(());
                                    }
                                }
                            }
                            info!("backend thread end {:?}", now.elapsed());
                            anyhow::Ok(())
                         }{
                            error!("error in run backend {err}");
                         }
                        }
                    }
                }
            });
        Ok(())
    }
    /// data was transfer from data container -> user container -> data container
    pub(crate) async fn route_data(
        &mut self,
        token: CancellationToken,
    ) -> Result<JoinSet<Result<()>>> {
        let (ipc_process_data_req_tx, mut ipc_process_data_req_rx) = mpsc::channel(1);
        self.ipc_process_data_req_tx = Some(ipc_process_data_req_tx);

        let (ipc_process_submit_result_tx, mut ipc_process_submit_result_rx) = mpsc::channel(1);
        self.ipc_process_submit_output_tx = Some(ipc_process_submit_result_tx);

        let (ipc_process_completed_data_tx, mut ipc_process_completed_data_rx) = mpsc::channel(1);
        self.ipc_process_completed_data_tx = Some(ipc_process_completed_data_tx);

        let (ipc_process_finish_state_tx, mut ipc_process_finish_state_rx) = mpsc::channel(1);
        self.ipc_process_finish_state_tx = Some(ipc_process_finish_state_tx);

        let mut join_set: JoinSet<Result<()>> = JoinSet::new();

        let new_data_tx = broadcast::Sender::new(1);
        if !self.downstreams.is_empty() {
            //process outgoing data
            {
                let new_data_tx = new_data_tx.clone();
                let token = token.clone();
                join_set.spawn(async move {
                    let mut interval = time::interval(Duration::from_secs(2));
                    loop {
                        select! {
                            _ = token.cancelled() => {
                               return anyhow::Ok(());
                            }
                            _ = interval.tick() => {
                                let _ = new_data_tx.send(());
                            }
                        }
                    }
                });
            }

            for _ in 0..10 {
                let downstreams = self.downstreams.clone();
                let mut multi_sender = MultiSender::new(downstreams.clone());
                let fs_cache = self.fs_cache.clone();
                let token = token.clone();
                let node_name = self.name.clone();
                let db_repo = self.repo.clone();

                let mut new_data_rx = new_data_tx.subscribe();

                join_set.spawn(async move {
                    loop {
                        tokio::select! {
                            _ = token.cancelled() => {
                                return anyhow::Ok(());
                             }
                           new_data = new_data_rx.recv() => {
                                if new_data.is_err() {
                                    //drop fast to provent exceed channel capacity
                                    continue;
                                }
                                //reconstruct batch
                                //TODO combine multiple batch
                                loop {
                                    let now = Instant::now();
                                    match db_repo.find_data_and_mark_state(&node_name, &Direction::Out, &DataState::SelectForSend).await {
                                       Ok(Some(req)) =>{
                                            let new_batch = match fs_cache.read(&req.id).await {
                                               Ok(batch) => batch,
                                                Err(err) => {
                                                    warn!("failed to read batch: {}", err);
                                                    //todo how to handle missing data
                                                    if let Err(err) = db_repo.update_state(&node_name, &req.id,  &Direction::Out, &DataState::Error, None).await {
                                                        error!("mark data {} to fail {}", &req.id, err);
                                                    }
                                                    break;
                                                }
                                            };

                                            //write outgoing
                                            if new_batch.size >0 && !downstreams.is_empty() {
                                                info!("start to send data {} {:?}", &req.id, now.elapsed());
                                                let sent_nodes: Vec<_>=  req.sent.iter().map(|v|v.as_str()).collect();
                                                if let Err(sent_nodes) =  multi_sender.send(new_batch, &sent_nodes).await {
                                                    if let Err(err) = db_repo.update_state(&node_name, &req.id, &Direction::Out,  &DataState::PartialSent, Some(sent_nodes.iter().map(|key|key.as_str()).collect())).await{
                                                        error!("revert data state fail {err}");
                                                    }
                                                    warn!("send data to partial downnstream {:?}",sent_nodes);
                                                    break;
                                                }

                                                info!("send data to downnstream successfully {} {:?}", &req.id, now.elapsed());
                                            }

                                            match db_repo.update_state(&node_name, &req.id,  &Direction::Out, &DataState::Sent, Some(downstreams.iter().map(|key|key.as_str()).collect())).await{
                                               Ok(_) =>{
                                                        //remove input data
                                                        if let Err(err) =  fs_cache.remove(&req.id).await {
                                                            error!("remove template batch fail {}", err);
                                                        }

                                                },
                                                Err(err) => {
                                                    error!("update state to process fail {} {}", &req.id, err)
                                                }
                                            }
                                            info!("fetch and send a batch {:?}", now.elapsed());
                                        },
                                       Ok(None)=>{
                                            break;
                                        }
                                        Err(err)=>{
                                            error!("insert  incoming data record to mongo {err}");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }

        {
            //process submit data
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let buf_size = self.buf_size;
            let token = token.clone();
            let local_state = self.local_state.clone();

            join_set.spawn(async move {
                loop {
                    select! {
                    _ = token.cancelled() => {
                        return anyhow::Ok(());
                        }
                     Some((req, resp))  = ipc_process_submit_result_rx.recv() => {
                        //check finish state
                        if matches!(*local_state.read().await, TrackerState::Finish) {
                            resp.send(Err(anyhow!("node was finished"))).expect("channel only read once");
                            continue;
                        }

                        loop {
                            if let Err(err) =  db_repo.count(&node_name, &[&DataState::Received, &DataState::PartialSent], &Direction::Out).await.and_then(|count|{
                                if count > buf_size {
                                    Err(anyhow!("has reach limit current:{count} limit:{buf_size}"))
                                } else {
                                    Ok(())
                                }
                            }){
                                warn!("fail with limit {err}");
                                sleep(Duration::from_secs(10)).await;
                                continue;
                            }
                            break;
                        }



                        info!("start to insert data {}", &req.id);
                        // respose with nothing
                        let tm =Utc::now().timestamp();
                        match db_repo.insert_new_path(&DataRecord{
                            node_name:node_name.clone(),
                            id:req.id.clone(),
                            size: req.size,
                            sent: vec![],
                            state: DataState::Received,
                            direction: Direction::Out,
                            created_at:tm,
                            updated_at:tm,
                        }).await{
                           Ok(_) =>{
                                // respose with nothing
                                resp.send(Ok(())).expect("channel only read once");
                                info!("insert data batch {}", req.id);
                                let _ = new_data_tx.send(());
                            },
                            Err(err) => {
                                resp.send(Err(err)).expect("channel only read once");
                            }
                        }
                    },
                    }
                }
            });
        }

        //TODO this make a async process to be sync process. got a low performance,
        //if have any bottleneck here, we should refrator this one
        if !self.upstreams.is_empty() {
            //process user contaienr request
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let url = self.upstreams[0].clone();
            let fs_cache = self.fs_cache.clone();
            let token = token.clone();
            let local_state = self.local_state.clone();

            join_set.spawn(async move {
                //    let mut client = DataStreamClient::connect(url.clone()).await;
                let mut client: Option<DataStreamClient<Channel>> = None;
                loop {
                    select! {
                      _ = token.cancelled() => {
                            return anyhow::Ok(());
                      }
                     Some((_, resp)) = ipc_process_data_req_rx.recv() => {
                            //check finish state
                            if matches!(*local_state.read().await, TrackerState::Finish) {
                                resp.send(Err(anyhow!("node was finished"))).expect("channel only read once");
                                continue;
                            }
                            //select a unassgined data
                            info!("try to find avaiable data");
                            if client.is_none() {
                                match DataStreamClient::connect(url.clone()).await{
                                   Ok(new_client)=>{
                                        client = Some(new_client)
                                    },
                                    Err(err) =>{
                                        error!("cannt connect upstream {err}");
                                        resp.send(Ok(None)).expect("channel only read once");
                                        continue;
                                    }
                                }
                            }

                            let client_non = client.as_mut().unwrap();
                            match client_non.request_media_data(Empty{}).await {
                               Ok(record) =>{
                                    let data = record.into_inner();
                                    match db_repo.find_by_node_id(&node_name,&data.id, &Direction::In).await  {
                                       Ok(Some(_))=>{
                                            resp.send(Ok(None)).expect("channel only read once");
                                            continue;
                                        }
                                        Err(err)=>{
                                            error!("query mongo by id {err}");
                                            resp.send(Err(err)).expect("request alread listen this channel");
                                            continue;
                                        }
                                       _=>{}
                                    }

                                    let res_data = AvaiableDataResponse{
                                        id:   data.id.clone(),
                                        size: data.size
                                    };

                                    if let Err(err) = fs_cache.write(data).await {
                                        error!("write cache files {err}");
                                        resp.send(Err(anyhow!("write cache files {err}"))).expect("channel only read once");
                                        continue;
                                    }
                                    //mark data as received
                                    //TODO bellow can combined
                                    //node_name, &res_data.id, Direction::In, DataState::Received
                                    let tm =Utc::now().timestamp();
                                    if let Err(err) =  db_repo.insert_new_path(&DataRecord{
                                        node_name:node_name.clone(),
                                        id :res_data.id.clone(),
                                        size: res_data.size,
                                        sent: vec![],
                                        state: DataState::Received,
                                        direction: Direction::In,
                                        updated_at:tm,
                                        created_at:tm,
                                    }).await{
                                        error!("mark data as client receive {err}");
                                        resp.send(Err(anyhow!("mark data as client receive {err}"))).expect("channel only read once");
                                        continue;
                                    }

                                    //insert a new incoming data record
                                    if let Err(err) =  db_repo.update_state(&(node_name.clone() +"-channel"), &res_data.id, &Direction::In, &DataState::EndRecieved, None).await{
                                        error!("mark data as client receive {err}");
                                        resp.send(Err(anyhow!("mark data as client receive {err}"))).expect("channel only read once");
                                        continue;
                                    }

                                    info!("get data batch {} to user", &res_data.id);
                                    //response this data's position
                                    resp.send(Ok(Some(res_data))).expect("channel only read once");
                                },
                                Err(status)=>{
                                    match status.code() {
                                        Code::NotFound =>{
                                            resp.send(Ok(None)).expect("channel only read once");
                                        },
                                        Code::DeadlineExceeded =>{
                                            client  = None;
                                            resp.send(Err(anyhow!("network deadline exceeded"))).expect("channel only read once");
                                        },
                                        Code::Unavailable =>{
                                            client  = None;
                                            resp.send(Err(anyhow!("connect break"))).expect("channel only read once");
                                        },
                                        _=>{
                                            error!("unable to retrieval data from channel {:?}", status);
                                            resp.send(Err(anyhow!("unable to retrieval data"))).expect("channel only read once");
                                        }
                                    }
                                }
                            }
                     },
                     Some((req, resp))  = ipc_process_completed_data_rx.recv() => {
                        //check finish state
                        if matches!(*local_state.read().await, TrackerState::Finish) {
                            resp.send(Err(anyhow!("node was finished"))).expect("channel only read once");
                            continue;
                        }
                        match db_repo.update_state(&node_name, &req.id, &Direction::In, &DataState::Processed, None).await{
                           Ok(_) =>{
                                    // respose with nothing
                                    resp.send(Ok(())).expect("channel only read once");
                                    if let Err(err) = fs_cache.remove(&req.id).await {
                                        error!("remove tmp fs fail {}", err);
                                        continue;
                                    }
                            },
                            Err(err) => {
                                resp.send(Err(err)).expect("channel only read once");
                            }
                        }
                     }
                    }
                }
            });
        }

        if self.upstreams.is_empty() {
            //receive event from pod
            //inputs nodes need to tell its finish
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let token = token.clone();
            join_set.spawn(async move {
                loop {
                    select! {
                      _ = token.cancelled() => {
                            return anyhow::Ok(());
                      }
                      Some((_, resp))  = ipc_process_finish_state_rx.recv() => {
                        loop {
                            //query data need to be done
                            let running_state = &[
                                &DataState::Received,
                                &DataState::Assigned,
                                &DataState::SelectForSend,
                                &DataState::PartialSent,
                                ];
                                match db_repo.count(&node_name, running_state.as_slice(), &Direction::Out).await {
                                    Ok(count) => {
                                        if count ==0 {
                                            break;
                                        }
                                        debug!("there are {count} data need to be sent");
                                    },
                                    Err(err) => error!("query node state {err}")
                                }
                                sleep(Duration::from_secs(5)).await;
                        }

                        match db_repo.update_node_by_name(&node_name, TrackerState::Finish).await{
                           Ok(_) =>{
                                // respose with nothing

                                resp.send(Ok(())).expect("channel only read once");
                            },
                            Err(err) => {
                                resp.send(Err(err)).expect("channel only read once");
                            }
                        }
                     }
                    }
                }
        });
        }
        Ok(join_set)
    }
}
