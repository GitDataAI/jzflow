use crate::ipc::{
    AvaiableDataResponse,
    CompleteDataReq,
    ErrorNumber,
    IPCError,
    RequetDataReq,
    SubmitOuputDataReq,
};
use anyhow::{
    anyhow,
    Result,
};

use chrono::Utc;
use jiaoziflow::{
    core::db::{
        DataFlag,
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
    utils::k8s_helper::get_machine_name,
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

    pub(crate) data_cache: Arc<dyn FileCache>,

    pub(crate) repo: R,

    pub(crate) local_state: Arc<RwLock<TrackerState>>,

    pub(crate) up_nodes: Vec<String>,

    pub(crate) incoming_streams: Vec<String>,

    pub(crate) downstreams: Vec<String>,

    // channel for process avaiable data request
    pub(crate) ipc_process_data_req_tx:
        Option<MessageSender<RequetDataReq, Option<AvaiableDataResponse>, IPCError>>,

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
        data_cache: Arc<dyn FileCache>,
        buf_size: usize,
        up_nodes: Vec<String>,
        incoming_streams: Vec<String>,
        downstreams: Vec<String>,
    ) -> Self {
        MediaDataTracker {
            data_cache,
            buf_size,
            name: name.to_string(),
            repo,
            local_state: Arc::new(RwLock::new(TrackerState::Init)),
            up_nodes,
            incoming_streams,
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
                let mut to_check_finish = true;
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
                            if to_check_finish && !up_nodes.is_empty() {
                                let is_all_success = try_join_all(up_nodes.iter().map(|node_name|db_repo.get_node_by_name(node_name))).await
                                .map_err(|err|anyhow!("query node data {err}"))?
                                .iter()
                                .all(|node| node.state == TrackerState::Finish);

                                if is_all_success && db_repo.count(&node_name,  &[&DataState::Received,&DataState::Assigned], Some(&Direction::In)).await? == 0 {
                                    db_repo.mark_incoming_finish(&node_name).await.map_err(|err|anyhow!("update node data {err}"))?;
                                    info!("incoming data was finished, not need to run backend");
                                    to_check_finish = false;
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

        let machine_name = get_machine_name();
        let mut join_set: JoinSet<Result<()>> = JoinSet::new();

        let new_data_tx = broadcast::Sender::new(1);
        if !self.downstreams.is_empty() {
            //process outgoing data
            {
                let new_data_tx = new_data_tx.clone();
                let token = token.clone();
                join_set.spawn(async move {
                    let mut interval = time::interval(Duration::from_secs(5));
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
                let data_cache = self.data_cache.clone();
                let token = token.clone();
                let node_name = self.name.clone();
                let db_repo = self.repo.clone();
                let machine_name = machine_name.clone();

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
                                    match db_repo.find_data_and_mark_state(&node_name, &Direction::Out, true, &DataState::SelectForSend, Some(machine_name.clone())).await {
                                       Ok(Some(req)) =>{
                                            let mut new_batch = match data_cache.read(&req.id).await {
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
                                            //TODO keep this data
                                            new_batch.data_flag = req.flag.to_bit();
                                            new_batch.priority = req.priority as u32;

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
                                                        if !req.flag.is_keep_data {
                                                            if let Err(err) =  data_cache.remove(&req.id).await {
                                                                error!("remove template batch fail {}", err);
                                                            }
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
                        if *local_state.read().await == TrackerState::Finish {
                            resp.send(Err(anyhow!("node was finished"))).expect("channel only read once");
                            continue;
                        }

                        loop {
                            if let Err(err) =  db_repo.count(&node_name, &[&DataState::Received, &DataState::PartialSent], Some(&Direction::Out)).await.and_then(|count|{
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
                            machine: "".to_string(),
                            flag: req.data_flag,
                            priority: req.priority,
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

        let incoming_data_tx = broadcast::Sender::new(1);
        if !self.incoming_streams.is_empty() {
            //fetch data from channel
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let url = self.incoming_streams[0].clone();
            let data_cache = self.data_cache.clone();
            let token = token.clone();
            let local_state = self.local_state.clone();
            {
                let incoming_data_tx = incoming_data_tx.clone();
                let token = token.clone();
                join_set.spawn(async move {
                    let mut interval = time::interval(Duration::from_secs(2));
                    loop {
                        select! {
                            _ = token.cancelled() => {
                               return anyhow::Ok(());
                            }
                            _ = interval.tick() => {
                                if matches!(*local_state.read().await, TrackerState::Finish) {
                                    warn!("node is finish exit fetch data ticker");
                                    return anyhow::Ok(());
                                }
                                let _ = incoming_data_tx.send(());
                            }
                        }
                    }
                });
            }

            let local_state = self.local_state.clone();
            let mut incoming_data_rx = incoming_data_tx.subscribe();
            let outgoing_stream = self.downstreams.clone();
            join_set.spawn(async move {
                let mut client: Option<DataStreamClient<Channel>> = None;
                loop {
                    select! {
                      _ = token.cancelled() => {
                            return anyhow::Ok(());
                      }
                      _ = incoming_data_rx.recv() => {
                            //check finish state
                            if *local_state.read().await == TrackerState::Finish {
                                warn!("node is finish exit fetch data from channel");
                                return anyhow::Ok(());
                            }

                            if client.is_none() {
                                match DataStreamClient::connect(url.clone()).await{
                                   Ok(new_client)=>{
                                        client = Some(new_client)
                                    },
                                    Err(err) =>{
                                        error!("cannt connect upstream {err}");
                                        continue;
                                    }
                                }
                            }

                            let client_ref = client.as_mut().unwrap();
                            match client_ref.request_media_data(Empty{}).await {
                               Ok(record) =>{
                                    let data = record.into_inner();
                                    let id = data.id.clone();
                                    let size = data.size;
                                    match db_repo.find_by_node_id(&node_name,&data.id, &Direction::In).await  {
                                       Ok(Some(_))=>{
                                            //already exit in this node
                                            continue;
                                        }
                                        Err(err)=>{
                                            error!("query mongo by id {err}");
                                            continue;
                                        }
                                       _=>{}
                                    }

                                    let data_flag = DataFlag::new_from_bit(data.data_flag);
                                    let priority  = data.priority as u8;
                                    if let Err(err) = data_cache.write(data).await {
                                        error!("write cache files {err}");
                                        continue;
                                    }

                                    //mark data as received, 
                                    //TODO transaction
                                    let tm =Utc::now().timestamp();
                                     if data_flag.is_transparent_data {
                                        //record as output, skip process
                                        if !outgoing_stream.is_empty() {
                                            if let Err(err) =  db_repo.insert_new_path(&DataRecord{
                                                node_name:node_name.clone(),
                                                id : id.clone(),
                                                size,
                                                sent: vec![],
                                                machine: "".to_string(),
                                                flag: data_flag.clone(),
                                                priority,
                                                state: DataState::Received,
                                                direction: Direction::Out,
                                                updated_at:tm,
                                                created_at:tm,
                                            }).await{
                                                error!("mark data as client receive {err}");
                                                continue;
                                            }
                                        }

                                        if let Err(err) =  db_repo.insert_new_path(&DataRecord{
                                            node_name:node_name.clone(),
                                            id : id.clone(),
                                            size,
                                            sent: vec![],
                                            flag: data_flag.clone(),
                                            machine: "".to_string(),
                                            priority,
                                            state: DataState::Processed,
                                            direction: Direction::In,
                                            updated_at:tm,
                                            created_at:tm,
                                        }).await{
                                            error!("mark data as client receive {err}");
                                            continue;
                                        };
                                        info!("transparent data received");
                                    } else if let Err(err) =  db_repo.insert_new_path(&DataRecord{
                                        node_name:node_name.clone(),
                                        id : id.clone(),
                                        size,
                                        sent: vec![],
                                        machine: "".to_string(),
                                        flag: data_flag.clone(),
                                        priority,
                                        state: DataState::Received,
                                        direction: Direction::In,
                                        updated_at:tm,
                                        created_at:tm,
                                    }).await{
                                        error!("mark data as client receive {err}");
                                        continue;
                                    }



                                    //insert a new incoming data record
                                    if let Err(err) =  db_repo.update_state(&(node_name.clone() +"-channel"), &id, &Direction::In, &DataState::EndRecieved, None).await{
                                        error!("mark data as client receive {err}");
                                        continue;
                                    }

                                    info!("save data batch {}", &id);
                                },
                                Err(status)=>{
                                    match status.code() {
                                        Code::NotFound =>{
                                            continue;
                                        },
                                        _=>{
                                            client  = None;
                                            error!("unable to retrieval data from channel {:?}", status);
                                        }
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
            let db_repo = self.repo.clone();
            let node_name = self.name.clone();
            let data_cache = self.data_cache.clone();
            let token = token.clone();
            let local_state = self.local_state.clone();
            let machine_name = machine_name.clone();

            let incoming_data_tx = incoming_data_tx.clone();
            join_set.spawn(async move {
                loop {
                    select! {
                      _ = token.cancelled() => {
                            return anyhow::Ok(());
                      }
                     Some((req, resp)) = ipc_process_data_req_rx.recv() => {
                            //check finish state
                            if let Some(id) = req.id {
                                let result = match db_repo.find_by_node_id(&node_name, &id, &Direction::In).await {
                                    Ok(Some(record)) => match data_cache.exit(&id).await {
                                        Ok(true) => Ok(Some(AvaiableDataResponse {
                                            id: record.id.clone(),
                                            size: record.size,
                                        })),
                                        Ok(false) => Err(IPCError::NodeError {
                                            code: ErrorNumber::DataMissing,
                                            msg: "".to_string(),
                                        }),
                                        Err(err) => Err(IPCError::UnKnown(err.to_string())),
                                    },
                                    Ok(None) => Ok(None),
                                    Err(err) => Err(IPCError::UnKnown(err.to_string())),
                                };
                                resp.send(result).expect("channel send failed: channel can only be read once");
                                continue;
                            }

                            if *local_state.read().await == TrackerState::Finish {
                                resp.send(Err(IPCError::NodeError {
                                    code: ErrorNumber::AlreadyFinish,
                                    msg: "node is already finish".to_string(),
                                })).expect("channel send failed: channel can only be read once");
                                continue;
                            }

                            if *local_state.read().await == TrackerState::InComingFinish {
                                resp.send(Err(IPCError::NodeError {
                                    code: ErrorNumber::InComingFinish,
                                    msg: "node is already finish".to_string(),
                                })).expect("channel send failed: channel can only be read once");
                                continue;
                            }

                            let result = {
                                // if a pod take a task but crash or some reason not complete it, this data will hang up.
                                // TODO, also record who take this task, pod must pick this task first when restart.
                                let record = db_repo
                                    .find_data_and_mark_state(&node_name, &Direction::In, false, &DataState::Assigned, Some(machine_name.clone()))
                                    .await
                                    .map_err(|err| anyhow!("query available data failed: {}", err))?;
                                if record.is_none() {
                                    let _ = incoming_data_tx.send(());
                                    Ok(None)
                                } else {
                                    let record = record.unwrap();
                                    let res_data = AvaiableDataResponse {
                                        id: record.id.clone(),
                                        size: record.size,
                                    };
                                    Ok(Some(res_data))
                                }
                            };
                            resp.send(result).expect("channel send failed: channel can only be read once");
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
                                    if let Err(err) = data_cache.remove(&req.id).await {
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

        {
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
                                match db_repo.count(&node_name, running_state.as_slice(), Some(&Direction::Out)).await {
                                    Ok(count) => {
                                        info!(count);
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
