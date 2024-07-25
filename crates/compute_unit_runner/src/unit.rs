use anyhow::Result;
use jz_action::network::datatransfer::data_stream_server::DataStream;
use jz_action::network::datatransfer::{MediaDataBatchResponse, TabularDataBatchResponse};
use jz_action::network::nodecontroller::node_controller_server::NodeController;
use jz_action::network::nodecontroller::{NodeType, StartRequest};
use jz_action::utils::AnyhowToGrpc;
use jz_action::{network::common::Empty, utils::StdIntoAnyhowResult};
use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};
use tracing::error;

use super::media_data_tracker::{MediaDataTracker, TrackerState};

pub struct DataNodeControllerServer {
    pub program: Arc<Mutex<MediaDataTracker>>,
}

#[tonic::async_trait]
impl NodeController for DataNodeControllerServer {
    async fn start(
        &self,
        request: Request<StartRequest>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let request = request.into_inner();
        let mut program_guard = self.program.lock().await;
        program_guard.state = TrackerState::Ready;
        program_guard.node_type = NodeType::try_from(request.node_type)
            .anyhow()
            .to_rpc(Code::InvalidArgument)?;
        program_guard.upstreams = Some(request.upstreams);
        program_guard.script = Some(request.script);

        program_guard
            .process_data_cmd()
            .await
            .to_rpc(Code::Internal)?;

        Ok(Response::new(Empty {}))
    }

    async fn pause(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().await;
        program_guard.state = TrackerState::Pending;
        Ok(Response::new(Empty {}))
    }

    async fn restart(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().await;
        program_guard.state = TrackerState::Ready;
        Ok(Response::new(Empty {}))
    }

    async fn stop(&self, _request: Request<Empty>) -> std::result::Result<Response<Empty>, Status> {
        let mut program_guard = self.program.lock().await;
        program_guard.state = TrackerState::Stopped;
        Ok(Response::new(Empty {}))
    }
}

pub struct UnitDataStream {
    pub program: Arc<Mutex<MediaDataTracker>>,
}

#[tonic::async_trait]
impl DataStream for UnitDataStream {
    type subscribe_media_dataStream = ReceiverStream<Result<MediaDataBatchResponse, Status>>;

    async fn subscribe_media_data(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Self::subscribe_media_dataStream>, Status> {
        println!("recieve new data request {:?}", request);

        let (tx, rx) = mpsc::channel(4);
        let program_guard = self.program.lock().await;
        let mut data_rx = program_guard.out_going_tx.subscribe();
        tokio::spawn(async move {
            loop {
                select! {
                 data_batch = data_rx.recv() => {
                    if let Err(e) = tx.send(data_batch.anyhow().to_rpc(Code::Internal)).await {
                        //todo handle disconnect
                        error!("send data {}", e);
                        return
                    }
                 },
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type subscribe_tabular_dataStream = ReceiverStream<Result<TabularDataBatchResponse, Status>>;

    async fn subscribe_tabular_data(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::subscribe_tabular_dataStream>, Status> {
        todo!()
    }
}
