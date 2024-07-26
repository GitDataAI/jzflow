use crate::ipc::{AvaiableDataResponse, SubmitResultReq};
use anyhow::Result;
use jz_action::core::models::{Node, NodeRepo, TrackerState};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_client::DataStreamClient;
use jz_action::network::datatransfer::{MediaDataBatchResponse, MediaDataCell};
use jz_action::network::nodecontroller::NodeType;
use mongodb::bson::doc;
use mongodb::{Collection, Database};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;
use walkdir::WalkDir;

use tokio::fs;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::sync::{broadcast, oneshot};
use tokio::time;
use tokio_stream::StreamExt;

use jz_action::utils::StdIntoAnyhowResult;

#[derive(Debug, PartialEq)]
pub enum DataStateEnum {
    Received,
    Assigned,
    Processed,
    Sent,
}

#[derive(Debug)]
pub struct BatchState {
    pub(crate) state: DataStateEnum,
}

pub struct MediaDataTracker<R>
where
    R: NodeRepo,
{
    pub(crate) name: String,

    pub(crate) tmp_store: PathBuf,

    pub(crate) repo: R,

    pub(crate) local_state: TrackerState,

    pub(crate) node_type: NodeType,

    pub(crate) upstreams: Option<Vec<String>>,

    pub(crate) ipc_process_submit_result_tx:
        Option<mpsc::Sender<(SubmitResultReq, oneshot::Sender<()>)>>,
    pub(crate) ipc_process_data_req_tx:
        Option<mpsc::Sender<((), oneshot::Sender<AvaiableDataResponse>)>>,
    pub(crate) out_going_tx: broadcast::Sender<MediaDataBatchResponse>, //receive data from upstream and send it to program with this
}

impl<R> MediaDataTracker<R>
where
    R: NodeRepo,
{
    pub fn new(repo: R, name: &str, tmp_store: PathBuf) -> Self {
        let out_going_tx = broadcast::Sender::new(128);

        MediaDataTracker {
            tmp_store,
            name: name.to_string(),
            repo,
            node_type: NodeType::Input,
            local_state: TrackerState::Init,
            upstreams: None,
            ipc_process_submit_result_tx: None,
            ipc_process_data_req_tx: None,
            out_going_tx: out_going_tx,
        }
    }

    pub(crate) async fn process_data_cmd(&mut self) -> Result<()> {
        match self.node_type {
            NodeType::Input => self.track_input_data().await,
            NodeType::InputOutput => self.track_input_output_data().await,
            NodeType::Output => self.track_output_data().await,
        }
    }

    pub(crate) async fn track_input_data(&mut self) -> Result<()> {
        let (ipc_process_submit_result_tx, mut ipc_process_submit_result_rx) = mpsc::channel(1024);
        self.ipc_process_submit_result_tx = Some(ipc_process_submit_result_tx);

        //TODO this make a async process to be sync process. got a low performance,
        //if have any bottleneck here, we should refrator this one
        let out_going_tx = self.out_going_tx.clone();
        let tmp_store = self.tmp_store.clone();

        let mut state_map: HashMap<String, BatchState> = HashMap::new();
        tokio::spawn(async move {
            loop {
                select! {
                 Some((req, resp))  = ipc_process_submit_result_rx.recv() => {
                    state_map.insert(req.id.clone(), BatchState{
                        state: DataStateEnum::Processed,
                    });

                    // respose with nothing
                    resp.send(()).expect("channel only read once");

                    let tmp_out_path = tmp_store.join(req.id.clone()+"-out");
                    let mut new_batch =MediaDataBatchResponse::default();

                    let mut entry_count = 0 ;
                    for entry in WalkDir::new(tmp_out_path) {
                        match entry {
                           Ok(entry) => {
                                if entry.file_type().is_file() {
                                    let path  = entry.path();
                                    match fs::read(path).await {
                                       Ok(content) => {
                                            new_batch.cells.push(MediaDataCell{
                                                size: content.len() as i32,
                                                path: path.to_str().unwrap().to_string(),
                                                data: content,
                                            });
                                            entry_count+=1;
                                        }
                                        Err(e) => error!("read file({:?}) fail {}", path, e),
                                    }

                                    println!("{}", entry.path().display());
                                }
                            }
                            Err(e) => error!("walk out dir({:?}) fail {}", &req.id, e),
                        }
                    }
                    new_batch.size  = entry_count;

                    //write outgoing
                    if new_batch.size >0 {
                        if let Err(e) = out_going_tx.send(new_batch) {
                            error!("send data {}", e);
                            continue;
                        }

                        let entry = state_map.get_mut(&req.id)
                        .expect("this value has been inserted before");
                        entry.state = DataStateEnum::Sent;
                    }
                    let _ = state_map.remove(&req.id);
                },
                }
            }
        });
        Ok(())
    }

    pub(crate) async fn track_input_output_data(&mut self) -> Result<()> {
        let upstreams = self
            .upstreams
            .as_ref()
            .expect("input output node must have incoming nodes");

        let (incoming_data_tx, mut incoming_data_rx) = mpsc::channel(1024);

        let (ipc_process_data_req_tx, mut ipc_process_data_req_rx) = mpsc::channel(1024);
        self.ipc_process_data_req_tx = Some(ipc_process_data_req_tx);

        let (ipc_process_submit_result_tx, mut ipc_process_submit_result_rx) = mpsc::channel(1024);
        self.ipc_process_submit_result_tx = Some(ipc_process_submit_result_tx);

        for upstream in upstreams {
            {
                let upstream = upstream.clone();
                let tx_clone = incoming_data_tx.clone();
                let _ = tokio::spawn(async move {
                    //todo handle network disconnect
                    let mut client = DataStreamClient::connect(upstream.clone()).await?;
                    let mut stream = client.subscribe_media_data(Empty {}).await?.into_inner();

                    while let Some(item) = stream.next().await {
                        tx_clone.send(item.unwrap()).await.unwrap();
                    }

                    error!("unable read data from stream");
                    anyhow::Ok(())
                });
            }
            info!("listen data from upstream {}", upstream);
        }

        //TODO this make a async process to be sync process. got a low performance,
        //if have any bottleneck here, we should refrator this one
        let out_going_tx = self.out_going_tx.clone();
        let tmp_store = self.tmp_store.clone();
        let mut state_map: HashMap<String, BatchState> = HashMap::new();
        tokio::spawn(async move {
            loop {
                select! {
                 data_batch_result = incoming_data_rx.recv() => {
                    if let Some(data_batch) = data_batch_result {
                        //create input directory
                        let id = Uuid::new_v4().to_string();
                        let tmp_in_path = tmp_store.join(id.clone()+"-input");
                        if let Err(e) = fs::create_dir_all(&tmp_in_path).await {
                            error!("create input dir {:?} fail {}", tmp_in_path, e);
                            return
                        }

                        //create output directory at the same time
                        let tmp_out_path = tmp_store.join(id.clone()+"-output");
                        if let Err(e) = fs::create_dir_all(&tmp_out_path).await {
                            error!("create output dir {:?} fail {}", tmp_out_path, e);
                            return
                        }
                        //write batch files
                        for entry in  data_batch.cells.iter() {
                            let entry_path = tmp_in_path.join(entry.path.clone());
                            if let Err(e) = fs::write(entry_path.clone(), &entry.data).await {
                                error!("write file {:?} fail {}", entry_path, e);
                            }
                        }
                        state_map.insert(id, BatchState{
                            state: DataStateEnum::Received,
                        });
                    }
                 },
                 Some((_, resp)) = ipc_process_data_req_rx.recv() => {
                        //select a unassgined data
                        for (key, v ) in  state_map.iter_mut() {
                            if v.state == DataStateEnum::Received {
                                //response this data's position
                                resp.send(AvaiableDataResponse{
                                    id:   key.clone(),
                                }).expect("channel only read once");
                                v.state = DataStateEnum::Assigned ;
                                break;
                            }
                        }
                 },
                 Some((req, resp))  = ipc_process_submit_result_rx.recv() => {
                    //mark this data as completed
                    match state_map.get_mut(&req.id) {
                        Some(state)=>{
                            state.state = DataStateEnum::Processed;
                        },
                        None=>error!("id({:?}) not found", &req.id)
                    }
                    // respose with nothing
                    resp.send(()).expect("channel only read once");

                    //reconstruct batch
                    //TODO combine multiple batch
                    let tmp_out_path = tmp_store.join(req.id.clone()+"-out");
                    let mut new_batch = MediaDataBatchResponse::default();

                    let mut entry_count = 0 ;
                    for entry in WalkDir::new(tmp_out_path) {
                        match entry {
                           Ok(entry) => {
                                if entry.file_type().is_file() {
                                    let path  = entry.path();
                                    match fs::read(path).await {
                                       Ok(content) => {
                                            new_batch.cells.push(MediaDataCell{
                                                size: content.len() as i32,
                                                path: path.to_str().unwrap().to_string(),
                                                data: content,
                                            });
                                            entry_count+=1;
                                        }
                                        Err(e) => error!("read file({:?}) fail {}", path, e),
                                    }

                                    println!("{}", entry.path().display());
                                }
                            }
                            Err(e) => error!("walk out dir({:?}) fail {}", &req.id, e),
                        }
                    }
                    new_batch.size  = entry_count;

                    //write outgoing
                    if new_batch.size >0 {
                        if let Err(e) = out_going_tx.send(new_batch) {
                            error!("send data {}", e);
                            continue;
                        }
                        let entry = state_map.get_mut(&req.id)
                        .expect("this value has been inserted before");
                        entry.state = DataStateEnum::Sent;
                    }

                    //remove data
                    let tmp_path = tmp_store.join(req.id.clone()+"-input");
                    if let Err(e) = fs::remove_dir(&tmp_path).await {
                        error!("remove tmp dir{:?} fail {}", tmp_path, e);
                    }
                    let _ = state_map.remove(&req.id);
                },
                }
            }
        });
        Ok(())
    }

    pub(crate) async fn track_output_data(&mut self) -> Result<()> {
        let upstreams = self
            .upstreams
            .as_ref()
            .expect("input output node must have incoming nodes");

        let (incoming_data_tx, mut incoming_data_rx) = mpsc::channel(1024);

        let (ipc_process_data_req_tx, mut ipc_process_data_req_rx) = mpsc::channel(1024);
        self.ipc_process_data_req_tx = Some(ipc_process_data_req_tx);

        let (ipc_process_submit_result_tx, mut ipc_process_submit_result_rx) = mpsc::channel(1024);
        self.ipc_process_submit_result_tx = Some(ipc_process_submit_result_tx);

        for upstream in upstreams {
            {
                let upstream = upstream.clone();
                let tx_clone = incoming_data_tx.clone();
                let _ = tokio::spawn(async move {
                    //todo handle network disconnect
                    let mut client = DataStreamClient::connect(upstream.clone()).await?;
                    let mut stream = client.subscribe_media_data(Empty {}).await?.into_inner();

                    while let Some(item) = stream.next().await {
                        tx_clone.send(item.unwrap()).await.unwrap();
                    }

                    error!("unable read data from stream");
                    anyhow::Ok(())
                });
            }
            info!("listen data from upstream {}", upstream);
        }

        //TODO this make a async process to be sync process. got a low performance,
        //if have any bottleneck here, we should refrator this one
        let tmp_store = self.tmp_store.clone();
        //todo use db to keep this data.
        let mut state_map: HashMap<String, BatchState> = HashMap::new();
        tokio::spawn(async move {
            loop {
                select! {
                 data_batch_result = incoming_data_rx.recv() => {
                    if let Some(data_batch) = data_batch_result {
                        //create input directory
                        let id = Uuid::new_v4().to_string();
                        let tmp_in_path = tmp_store.join(id.clone()+"-input");
                        if let Err(e) = fs::create_dir_all(&tmp_in_path).await {
                            error!("create input dir {:?} fail {}", tmp_in_path, e);
                            return
                        }

                        //create output directory at the same time
                        let tmp_out_path = tmp_store.join(id.clone()+"-output");
                        if let Err(e) = fs::create_dir_all(&tmp_out_path).await {
                            error!("create output dir {:?} fail {}", tmp_out_path, e);
                            return
                        }
                        //write batch files
                        for entry in  data_batch.cells.iter() {
                            let entry_path = tmp_in_path.join(entry.path.clone());
                            if let Err(e) = fs::write(entry_path.clone(), &entry.data).await {
                                error!("write file {:?} fail {}", entry_path, e);
                            }
                        }
                        state_map.insert(id, BatchState{
                            state: DataStateEnum::Received,
                        });
                    }
                 },
                 Some((_, resp)) = ipc_process_data_req_rx.recv() => {
                        //select a unassgined data
                        for (key, v ) in  state_map.iter_mut() {
                            if v.state == DataStateEnum::Received {
                                //response this data's position
                                resp.send(AvaiableDataResponse{
                                    id:   key.clone(),
                                }).expect("channel only read once");
                                v.state = DataStateEnum::Assigned ;
                                break;
                            }
                        }
                 },
                 Some((req, _resp))  = ipc_process_submit_result_rx.recv() => {
                    //mark this data as completed
                    match state_map.get_mut(&req.id) {
                        Some(state)=>{
                            state.state = DataStateEnum::Processed;
                        },
                        None=>error!("id({:?}) not found", &req.id)
                    }
                    //remove data
                    let tmp_path = tmp_store.join(req.id.clone()+"-input");
                    if let Err(e) = fs::remove_dir(&tmp_path).await {
                        error!("remove tmp dir{:?} fail {}", tmp_path, e);
                    }
                    //remove state
                    let _ = state_map.remove(&req.id);
                },
                }
            }
        });
        Ok(())
    }

    pub(crate) async fn pool_state(
        database: mongodb::Database,
        name: &str,
        program: Arc<Mutex<MediaDataTracker<R>>>,
    ) -> Result<()> {
        let col: Collection<Node> = database.collection("node_state");
        let mut interval = time::interval(time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let record = col
                .find_one(doc! { "name": name })
                .await?
                .expect("record has inserted in controller");
            match record.state {
                TrackerState::Ready => {
                    let mut program_guard = program.lock().await;

                    if matches!(program_guard.local_state, TrackerState::Init) {
                        //start
                        program_guard.local_state = TrackerState::Ready;
                        program_guard.node_type =
                            NodeType::try_from(record.input_output_type).anyhow()?;
                        program_guard.upstreams = Some(record.upstreams);
                        program_guard.process_data_cmd().await?;
                    }
                }
                TrackerState::Stop => {
                    todo!()
                }
                _ => {}
            }
        }
        Ok(())
    }
}
