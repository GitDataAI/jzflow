use anyhow::Result;
use jz_action::core::models::{DbRepo, NodeRepo};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_server::DataStream;
use jz_action::network::datatransfer::{MediaDataBatchResponse, TabularDataBatchResponse};
use jz_action::utils::{AnyhowToGrpc, IntoAnyhowResult};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};
use tracing::info;

use super::channel_tracker::ChannelTracker;

pub struct ChannelDataStream<R>
where
    R: DbRepo,
{
    pub(crate) program: Arc<Mutex<ChannelTracker<R>>>,
}

#[tonic::async_trait]
impl<R> DataStream for ChannelDataStream<R>
where
    R: DbRepo,
{
    async fn transfer_media_data(
        &self,
        req: Request<MediaDataBatchResponse>,
    ) -> Result<Response<Empty>, Status> {
        let send_tx = {
            let program = self.program.lock().await;
            if program.receiver_rx.is_none() {
                return Err(Status::internal("not ready"));
            }
            program.receiver_rx.as_ref().unwrap().clone()
        };

        let (tx, rx) = oneshot::channel::<Result<()>>();
        let req: MediaDataBatchResponse = req.into_inner();
        if let Err(err) = send_tx.send((req, tx)).await {
            return Err(Status::from_error(Box::new(err)));
        }

        match rx.await {
            Ok(Ok(_)) => Ok(Response::new(Empty {})),
            Ok(Err(err)) => Err(Status::internal(err.to_string())),
            Err(err) => Err(Status::from_error(Box::new(err))),
        }
    }
    async fn transfer_tabular_data(
        &self,
        req: Request<TabularDataBatchResponse>,
    ) -> Result<Response<Empty>, tonic::Status> {
        todo!()
    }
}
