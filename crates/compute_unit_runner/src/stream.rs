use anyhow::Result;
use jz_action::core::models::NodeRepo;
use jz_action::network::datatransfer::data_stream_server::DataStream;
use jz_action::network::datatransfer::{MediaDataBatchResponse, TabularDataBatchResponse};
use jz_action::utils::AnyhowToGrpc;
use jz_action::{network::common::Empty, utils::StdIntoAnyhowResult};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};
use tracing::{error, info};

use super::media_data_tracker::MediaDataTracker;

pub struct UnitDataStream<R>
where
    R: NodeRepo + Send + Sync + 'static,
{
    pub program: Arc<Mutex<MediaDataTracker<R>>>,
    _phantom: PhantomData<R>,
}

impl<R> UnitDataStream<R>
where
    R: NodeRepo + Send + Sync + 'static,
{
    pub fn new(program: Arc<Mutex<MediaDataTracker<R>>>) -> Self {
        UnitDataStream {
            program,
            _phantom: PhantomData,
        }
    }
}

#[tonic::async_trait]
impl<R> DataStream for UnitDataStream<R>
where
    R: NodeRepo + Send + Sync + 'static,
{
    type subscribeMediaDataStream = ReceiverStream<Result<MediaDataBatchResponse, Status>>;
    async fn subscribe_media_data(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Self::subscribeMediaDataStream>, Status> {
        info!("receive media subscribe request {:?}", request);

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

    type subscribeTabularDataStream = ReceiverStream<Result<TabularDataBatchResponse, Status>>;

    async fn subscribe_tabular_data(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::subscribeTabularDataStream>, Status> {
        todo!()
    }
}
