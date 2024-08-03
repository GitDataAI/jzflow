use crate::fs_cache::FileCache;
use crate::ipc::{AvaiableDataResponse, CompleteDataReq, SubmitOuputDataReq};
use anyhow::{anyhow, Ok, Result};
use jz_action::core::models::{DataRecord, DataState, DbRepo, Direction, NodeRepo, TrackerState};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_client::DataStreamClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tonic::transport::Channel;
use tonic::Code;
use tracing::{debug, error, info, warn};

use crate::multi_sender::MultiSender;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tokio::time::{self, sleep, Instant};
use tokio_stream::StreamExt;

pub struct MediaDataTracker<R>
where
    R: DbRepo,
{
    pub(crate) name: String,

    pub(crate) buf_size: usize,

    pub(crate) fs_cache: Arc<dyn FileCache>,

    pub(crate) repo: R,

    pub(crate) local_state: TrackerState,

    pub(crate) upstreams: Vec<String>,

    pub(crate) downstreams: Vec<String>,

    // channel for process avaiable data request
    pub(crate) ipc_process_data_req_tx:
        Option<mpsc::Sender<((), oneshot::Sender<Result<Option<AvaiableDataResponse>>>)>>,

    // channel for response complete data. do clean work when receive this request
    pub(crate) ipc_process_completed_data_tx:
        Option<mpsc::Sender<(CompleteDataReq, oneshot::Sender<Result<()>>)>>,

    // channel for submit output data
    pub(crate) ipc_process_submit_output_tx:
        Option<mpsc::Sender<(SubmitOuputDataReq, oneshot::Sender<Result<()>>)>>,
}
impl<R> MediaDataTracker<R>
where
    R: DbRepo,
{
    pub fn new(repo: R, name: &str, fs_cache: Arc<dyn FileCache>, buf_size: usize) -> Self {
        MediaDataTracker {
            fs_cache,
            buf_size,
            name: name.to_string(),
            repo: repo,
            local_state: TrackerState::Init,
            upstreams: vec![],
            downstreams: vec![],
            ipc_process_submit_output_tx: None,
            ipc_process_completed_data_tx: None,
            ipc_process_data_req_tx: None,
        }
    }
}

impl<R> MediaDataTracker<R>
where
    R: DbRepo,
{
    /// data was transfer from data container -> user container -> data container
    pub(crate) async fn process_data_cmd(&mut self) -> Result<()> {
        let (ipc_process_data_req_tx, mut ipc_process_data_req_rx) = mpsc::channel(1);
        self.ipc_process_data_req_tx = Some(ipc_process_data_req_tx);

        let (ipc_process_submit_result_tx, mut ipc_process_submit_result_rx) = mpsc::channel(1);
        self.ipc_process_submit_output_tx = Some(ipc_process_submit_result_tx);

        let (ipc_process_completed_data_tx, mut ipc_process_completed_data_rx) = mpsc::channel(1);
        self.ipc_process_completed_data_tx = Some(ipc_process_completed_data_tx);

        let (new_data_tx, mut new_data_rx) = mpsc::channel(1);
        {
            //process outgoing data
            let db_repo = self.repo.clone();
            let downstreams = self.downstreams.clone(); //make dynamic downstreams?
            let node_name = self.name.clone();
            let new_data_tx = new_data_tx.clone();

            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(2));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            match new_data_tx.try_send(()) {
                                Err(TrySendError::Closed(_)) =>{
                                    error!("new data channel has been closed")
                                }
                                _=>{}
                            }
                        }
                    }
                }
            });

            let mut multi_sender = MultiSender::new(downstreams.clone());
            let fs_cache = self.fs_cache.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        Some(()) = new_data_rx.recv() => {
                            info!("try to send data");
                            //reconstruct batch
                            //TODO combine multiple batch
                            loop {
                                let now = Instant::now();
                                match db_repo.find_data_and_mark_state(&node_name, Direction::Out, DataState::SelectForSend).await {
                                    std::result::Result::Ok(Some(req)) =>{
                                        let new_batch = match fs_cache.read(&req.id).await {
                                            std::result::Result::Ok(batch) => batch,
                                            Err(err) => {
                                                warn!("Failed to read batch: {}", err);
                                                //todo how to handle missing data
                                                if let Err(err) = db_repo.update_state(&node_name, &req.id,  Direction::Out, DataState::Error, None).await {
                                                    error!("mark data {} to fail {}", &req.id, err);
                                                }
                                                break;
                                            }
                                        };

                                        //write outgoing
                                        if new_batch.size >0 && downstreams.len()>0 {
                                            info!("start to send data {} {:?}", &req.id, now.elapsed());
                                            let sent_nodes: Vec<_>=  req.sent.iter().map(|v|v.as_str()).collect();
                                            if let Err(sent_nodes) =  multi_sender.send(new_batch, &sent_nodes).await {
                                                if let Err(err) = db_repo.update_state(&node_name, &req.id, Direction::Out,  DataState::PartialSent, Some(sent_nodes.iter().map(|key|key.as_str()).collect())).await{
                                                    error!("revert data state fail {err}");
                                                }
                                                warn!("send data to partial downnstream {:?}",sent_nodes);
                                                break;
                                            }

                                            info!("send data to downnstream successfully {} {:?}", &req.id, now.elapsed());
                                        }

                                        match db_repo.update_state(&node_name, &req.id,  Direction::Out,DataState::Sent, Some(downstreams.iter().map(|key|key.as_str()).collect())).await{
                                            std::result::Result::Ok(_) =>{
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
                                    std::result::Result::Ok(None)=>{
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

        {
            //process submit data
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let buf_size = self.buf_size;

            tokio::spawn(async move {
                loop {
                    select! {
                     Some((req, resp))  = ipc_process_submit_result_rx.recv() => {
                        loop {
                            if let Err(err) =  db_repo.count_pending(&node_name,Direction::Out).await.and_then(|count|{
                                if count > buf_size {
                                    Err(anyhow!("has reach limit current:{count} limit:{buf_size}"))
                                } else {
                                    info!("ggggggggggggg current:{count} limit:{buf_size}");
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
                        match db_repo.insert_new_path(&DataRecord{
                            node_name:node_name.clone(),
                            id:req.id.clone(),
                            size: req.size,
                            sent: vec![],
                            state: DataState::Received,
                            direction: Direction::Out,
                        }).await{
                            std::result::Result::Ok(_) =>{
                                // respose with nothing
                                resp.send(Ok(())).expect("channel only read once");
                                info!("insert data batch {}", req.id);
                                match new_data_tx.try_send(()) {
                                    Err(TrySendError::Closed(_)) =>{
                                        error!("new data channel has been closed")
                                    }
                                    _=>{}
                                }
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
        if self.upstreams.len() > 0 {
            //process user contaienr request
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let url = self.upstreams[0].clone();
            let fs_cache = self.fs_cache.clone();

            tokio::spawn(async move {
                //    let mut client = DataStreamClient::connect(url.clone()).await;

                let mut client: Option<DataStreamClient<Channel>> = None;
                loop {
                    select! {
                     Some((_, resp)) = ipc_process_data_req_rx.recv() => {
                            //select a unassgined data
                            info!("try to find avaiable data");
                            if client.is_none() {
                                match DataStreamClient::connect(url.clone()).await{
                                    core::result::Result::Ok(new_client)=>{
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
                                std::result::Result::Ok(record) =>{
                                    let data = record.into_inner();
                                    let res_data = AvaiableDataResponse{
                                        id:   data.id.clone(),
                                        size: data.size,
                                    };

                                    if let Err(err) = fs_cache.write(data).await {
                                        error!("write cache files {err}");
                                        resp.send(Err(anyhow!("write cache files {err}"))).expect("channel only read once");
                                        continue;
                                    }
                                    //mark data as received
                                    //TODO bellow can combined
                                    //node_name, &res_data.id, Direction::In, DataState::Received
                                    if let Err(err) =  db_repo.insert_new_path(&DataRecord{
                                        node_name:node_name.clone(),
                                        id :res_data.id.clone(),
                                        size: res_data.size,
                                        sent: vec![],
                                        state: DataState::Received,
                                        direction: Direction::In,
                                    }).await{
                                        error!("mark data as client receive {err}");
                                        resp.send(Err(anyhow!("mark data as client receive {err}"))).expect("channel only read once");
                                        continue;
                                    }

                                    //insert a new incoming data record
                                    if let Err(err) =  db_repo.update_state(&(node_name.clone() +"-channel"), &res_data.id, Direction::In, DataState::EndRecieved,None).await{
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
                        match db_repo.update_state(&node_name, &req.id, Direction::In, DataState::Processed, None).await{
                            std::result::Result::Ok(_) =>{
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

        Ok(())
    }

    pub async fn apply_db_state(
        repo: R,
        name: &str,
        program: Arc<Mutex<MediaDataTracker<R>>>,
    ) -> Result<()> {
        let mut interval = time::interval(time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let record = repo
                .get_node_by_name(&name)
                .await
                .expect("record has inserted in controller or network error");
            debug!("{} fetch state from db", record.node_name);
            match record.state {
                TrackerState::Ready => {
                    let mut program_guard = program.lock().await;

                    if matches!(program_guard.local_state, TrackerState::Init) {
                        //start
                        info!("set to ready state {:?}", record.upstreams);
                        program_guard.local_state = TrackerState::Ready;
                        program_guard.upstreams = record.upstreams;
                        program_guard.downstreams = record.downstreams;
                        program_guard.process_data_cmd().await?;
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
