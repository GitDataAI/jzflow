use crate::ipc::{AvaiableDataResponse, CompleteDataReq, SubmitOuputDataReq};
use crate::mprc::Mprs;
use anyhow::{Ok, Result};
use jz_action::core::models::{DataRecord, DataState, DbRepo, Direction, NodeRepo, TrackerState};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_client::DataStreamClient;
use jz_action::network::datatransfer::{MediaDataBatchResponse, MediaDataCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tonic::Status;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use crate::multi_sender::MultiSender;
use tokio::fs;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::sync::{broadcast, oneshot};
use tokio::time::{self, sleep, Instant};
use tokio_stream::StreamExt;

pub struct MediaDataTracker<R>
where
    R: DbRepo,
{
    pub(crate) name: String,

    pub(crate) tmp_store: PathBuf,

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
    pub fn new(repo: R, name: &str, tmp_store: PathBuf) -> Self {
        MediaDataTracker {
            tmp_store,
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
            let tmp_store = self.tmp_store.clone();
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
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        Some(()) = new_data_rx.recv() => {
                            //reconstruct batch
                            //TODO combine multiple batch
                            loop {
                                let now = Instant::now();
                                match db_repo.find_and_sent_output_data(&node_name).await {
                                    std::result::Result::Ok(Some(req)) =>{
                                let tmp_out_path = tmp_store.join(&req.id);
                                let mut new_batch = MediaDataBatchResponse::default();
                                for entry in WalkDir::new(&tmp_out_path) {
                                    match entry {
                                        std::result::Result::Ok(entry) => {
                                            if entry.file_type().is_file() {
                                                let mut path  = entry.path();
                                                match fs::read(path).await {
                                                    std::result::Result::Ok(content) => {
                                                        path = path.strip_prefix(&tmp_out_path).expect("file is in the folder");
                                                        new_batch.cells.push(MediaDataCell{
                                                            size: content.len() as i32,
                                                            path: path.to_str().unwrap().to_string(),
                                                            data: content,
                                                        });
                                                    }
                                                    Err(err) => error!("read file({:?}) fail {err}", path),
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            error!("walk out dir({:?}) fail {err}", &req.id);
                                            if let Err(err) = db_repo.update_state(&node_name, &req.id, DataState::Error).await{
                                                error!("mark data state error fail {err}");
                                            }
                                            break;
                                        },
                                    }
                                }
                                new_batch.size  = req.size;
                                new_batch.id  = req.id.clone();

                                 //write outgoing
                                if new_batch.size >0 && downstreams.len()>0 {
                                    info!("start to send data {} {:?}", &req.id, now.elapsed());
                                    if let Err(sent_nodes) =  multi_sender.send(new_batch).await {
                                        if let Err(err) = db_repo.mark_partial_sent(&node_name, &req.id, sent_nodes.iter().map(|key|key.as_str()).collect()).await{
                                            error!("revert data state fail {err}");
                                        }
                                        warn!("send data to partial downnstream {:?}",sent_nodes);
                                        break;
                                    }

                                    info!("send data to downnstream successfully {} {:?}", &req.id, now.elapsed());
                                }

                                match db_repo.update_state(&node_name, &req.id, DataState::Sent).await{
                                    std::result::Result::Ok(_) =>{
                                            //remove input data
                                            let tmp_path = tmp_store.join(&req.id);
                                            if let Err(err) = fs::remove_dir_all(&tmp_path).await {
                                                error!("remove tmp dir{:?} fail {}", tmp_path, err);
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

        //TODO this make a async process to be sync process. got a low performance,
        //if have any bottleneck here, we should refrator this one
        {
            //process user contaienr request
            let tmp_store = self.tmp_store.clone();
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();

            tokio::spawn(async move {
                loop {
                    select! {
                     Some((_, resp)) = ipc_process_data_req_rx.recv() => {
                            //select a unassgined data
                            info!("try to find avaiable data");
                            match db_repo.find_and_assign_input_data(&node_name).await {
                                std::result::Result::Ok(Some(record)) =>{
                                    info!("get data batch {} to user", &record.id);
                                        //response this data's position
                                        resp.send(Ok(Some(AvaiableDataResponse{
                                            id:   record.id.clone(),
                                            size: record.size,
                                        }))).expect("channel only read once");
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
                     Some((req, resp))  = ipc_process_completed_data_rx.recv() => {
                        match db_repo.update_state(&node_name, &req.id, DataState::Processed).await{
                            std::result::Result::Ok(_) =>{
                                    // respose with nothing
                                    resp.send(Ok(())).expect("channel only read once");
                                    //remove input data
                                    let tmp_path = tmp_store.join(req.id.clone());
                                    if let Err(err) = fs::remove_dir_all(&tmp_path).await {
                                        error!("remove tmp dir{:?} fail {}", tmp_path, err);
                                    }
                            },
                            Err(err) => {
                                resp.send(Err(err)).expect("channel only read once");
                            }
                        }
                     },
                     Some((req, resp))  = ipc_process_submit_result_rx.recv() => {
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
                                println!("send signal");
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
