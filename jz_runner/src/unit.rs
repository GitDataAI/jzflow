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
    sync::Arc,
};
use tokio::sync::Mutex;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};
use tracing::error;

use super::program::{BatchProgram, ProgramState};

pub(crate) struct DataNodeControllerServer {
    pub(crate) program: Arc<Mutex<BatchProgram>>,
}


#[tonic::async_trait]
impl NodeController for DataNodeControllerServer {
    async fn start(
        &self,
        request: Request<StartRequest>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let request = request.into_inner();
        let mut program_guard = self.program.lock().await;
        program_guard.state = ProgramState::Ready;
        program_guard.upstreams = Some(request.upstreams);
        program_guard.script = Some(request.script);
        
        program_guard.fetch_data().await.to_rpc(Code::Internal)?;

        Ok(Response::new(Empty {}))
    }

    async fn pause(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().await;
        program_guard.state = ProgramState::Pending;
        Ok(Response::new(Empty {}))
    }

    async fn restart(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().await;
        program_guard.state = ProgramState::Ready;
        Ok(Response::new(Empty {}))
    }

    async fn stop(&self, _request: Request<Empty>) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().await;
        program_guard.state = ProgramState::Stopped;
        Ok(Response::new(Empty {}))
    }
}

pub(crate) struct UnitDataStream {
    pub(crate) program: Arc<Mutex<BatchProgram>>,
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
        let program_guard = self.program.lock().await;
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
