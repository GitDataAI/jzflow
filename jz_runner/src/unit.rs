use anyhow::{anyhow, Result};
use jz_action::network::datatransfer::data_stream_server::DataStream;
use jz_action::network::datatransfer::DataBatchResponse;
use jz_action::network::nodecontroller::node_controller_server::NodeController;
use jz_action::network::nodecontroller::StartRequest;
use jz_action::utils::AnyhowToGrpc;
use jz_action::{network::common::Empty, utils::StdIntoAnyhowResult};
use std::process::Command;
use std::{
    os::unix::process::CommandExt,
    sync::{Arc, Mutex},
};
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};
use tracing::error;

#[derive(Debug)]
pub(crate) enum ProgramState {
    Init,
    Ready,
    Success,
    Pending,
    Stop,
    Fail,
}

pub(crate) struct BatchProgram {
    pub(crate) state: ProgramState,
    pub(crate) script: Option<String>,

    pub(crate) tx: broadcast::Sender<DataBatchResponse>, //receive data from upstream and send it to program with this
}

impl BatchProgram {
    fn new() -> Self {
        let tx= broadcast::Sender::new(128);
        BatchProgram {
            state: ProgramState::Init,
            script: None,
            tx: tx,
        }
    }

    fn run_one_batch(&self) -> Result<()> {
        if self.script.is_none() {
            return Err(anyhow!("script not found"));
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(self.script.clone().unwrap())
            .output()?;
        if !output.status.success() {
            return Err(anyhow!("{}", String::from_utf8_lossy(&output.stderr)));
        }
        Ok(())
    }
}
pub(crate) struct DataNodeControllerServer {
    pub(crate) program: Arc<Mutex<BatchProgram>>,
}

impl DataNodeControllerServer {
    pub(crate) fn new() -> Self {
        DataNodeControllerServer {
            program: Arc::new(Mutex::new(BatchProgram::new())),
        }
    }
}

#[tonic::async_trait]
impl NodeController for DataNodeControllerServer {
    async fn start(
        &self,
        request: Request<StartRequest>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let request = request.into_inner();
        let mut program_guard = self.program.lock().anyhow().to_rpc(Code::Aborted)?;
        program_guard.state = ProgramState::Ready;
        program_guard.script = Some(request.script);
        Ok(Response::new(Empty {}))
    }

    async fn pause(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().anyhow().to_rpc(Code::Aborted)?;
        program_guard.state = ProgramState::Pending;
        Ok(Response::new(Empty {}))
    }

    async fn restart(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().anyhow().to_rpc(Code::Aborted)?;
        program_guard.state = ProgramState::Ready;
        Ok(Response::new(Empty {}))
    }

    async fn stop(&self, _request: Request<Empty>) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().anyhow().to_rpc(Code::Aborted)?;
        program_guard.state = ProgramState::Stop;
        Ok(Response::new(Empty {}))
    }
}

pub(crate) struct UnitDataStream {
    pub(crate) program: Arc<Mutex<BatchProgram>>,
}

impl UnitDataStream {
    pub(crate) fn new() -> Self {
        UnitDataStream {
            program: Arc::new(Mutex::new(BatchProgram::new())),
        }
    }
}

#[tonic::async_trait]
impl DataStream for UnitDataStream {
    type subscribe_new_dataStream = ReceiverStream<Result<DataBatchResponse, Status>>;

    async fn subscribe_new_data(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Self::subscribe_new_dataStream>, Status> {
        println!("recieve new data request {:?}", request);

        let (tx, rx) = mpsc::channel(4);
        let program_guard = self.program.lock().anyhow().to_rpc(Code::Aborted)?;
        let mut data_rx = program_guard.tx.subscribe();
        tokio::spawn(async move {
            loop {
                select! {
                 data_batch = data_rx.recv() => {
                    if let Err(e) = tx.send(data_batch.anyhow().to_rpc(Code::Internal)).await {
                        error!("send data {}", e);
                        return
                    }
                 },
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
