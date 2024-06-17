use anyhow::anyhow;
use jz_action::{network::common::Empty, utils::StdIntoAnyhowResult};
use jz_action::network::datatransfer::data_stream_server::DataStream;
use jz_action::network::datatransfer::DataBatchResponse;
use jz_action::network::nodecontroller::node_controller_server::NodeController;
use jz_action::network::nodecontroller::StartRequest;
use tokio::sync::mpsc;
use std::{os::unix::process::CommandExt, result::Result, sync::{Arc, Mutex}};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Code};
use std::process::Command;
use jz_action::utils::AnyhowToGrpc;
pub(crate) struct DataNodeControllerServer {
   pub(crate) child: Arc<Mutex<Option<std::process::Child>>>
}

#[tonic::async_trait]
impl NodeController for DataNodeControllerServer {
    async fn start(&self, request: Request<StartRequest>) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let child = Command::new("sh").
        arg("-c").arg(req.script).spawn()?;

        let mut guard = self.child.lock().anyhow().to_rpc(Code::Aborted)?;
        match guard.as_mut() {
            None => {
                *guard = Some(child);
                Ok(())
            },
            _=> Err(anyhow!("process is already running")),
        }.to_rpc(Code::Internal)?;

       Ok( Response::new(Empty{}))
    }
    async fn pause(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        let mut guard = self.child.lock().anyhow().to_rpc(Code::Aborted)?;

        if guard.is_none() {
            return  Ok( Response::new(Empty{}));
        }

        if let Some(child) = guard.as_mut() {
            let _ = child.wait()?;
        }
        *guard = None;
        Ok( Response::new(Empty{}))
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
    type subscribe_new_dataStream = ReceiverStream<Result<DataBatchResponse, Status>>;

    async fn subscribe_new_data(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::subscribe_new_dataStream>, Status> {
        todo!()
    }
}
