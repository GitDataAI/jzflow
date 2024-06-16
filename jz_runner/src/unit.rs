use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_server::DataStream;
use jz_action::network::datatransfer::DataBatch;
use jz_action::network::nodecontroller::node_controller_server::NodeController;

use std::result::Result;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

#[derive(Default)]
pub(crate) struct DataNodeControllerServer {}

#[tonic::async_trait]
impl NodeController for DataNodeControllerServer {
    async fn pause(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        todo!()
    }
    async fn restart(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        todo!()
    }
    async fn stop(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        todo!()
    }
}

#[derive(Default)]
pub(crate) struct UnitDataStrean {}

#[tonic::async_trait]
impl DataStream for UnitDataStrean {
    type subscribe_new_dataStream = ReceiverStream<Result<DataBatch, Status>>;

    async fn subscribe_new_data(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::subscribe_new_dataStream>, Status> {
        todo!()
    }
}
