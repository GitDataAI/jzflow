use anyhow::Result;
use jz_action::core::models::NodeRepo;
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_server::DataStream;
use jz_action::network::datatransfer::{MediaDataBatchResponse, TabularDataBatchResponse};
use jz_action::utils::{AnyhowToGrpc, IntoAnyhowResult};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};
use tracing::info;

use super::channel_tracker::ChannelTracker;

pub struct ChannelDataStream<R>
where
    R: NodeRepo + Send + Sync + 'static,
{
    pub(crate) program: Arc<Mutex<ChannelTracker<R>>>,
}

#[tonic::async_trait]
impl<R> DataStream for ChannelDataStream<R>
where
    R: NodeRepo + Send + Sync + 'static,
{
    type subscribeMediaDataStream = ReceiverStream<Result<MediaDataBatchResponse, Status>>;

    async fn subscribe_media_data(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Self::subscribeMediaDataStream>, Status> {
        info!("receive media subscribe request {:?}", request);
        let remote_addr = request
            .remote_addr()
            .anyhow("remove addr missing")
            .to_rpc(Code::InvalidArgument)?
            .to_string();

        let (tx, rx) = mpsc::channel(4);
        let program_guard = self.program.lock().await;
        let _ = program_guard.receivers.lock().await.insert(remote_addr, tx);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type subscribeTabularDataStream = ReceiverStream<Result<TabularDataBatchResponse, Status>>;

    async fn subscribe_tabular_data(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::subscribeTabularDataStream>, Status> {
        todo!()
    }
}
