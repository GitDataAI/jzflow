use anyhow::Result;
use jz_action::core::models::DbRepo;
use jz_action::network::datatransfer::data_stream_server::DataStream;
use jz_action::network::datatransfer::{MediaDataBatchResponse, TabularDataBatchResponse};
use jz_action::utils::{AnyhowToGrpc, IntoAnyhowResult};
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
    R: DbRepo + Clone + Send + Sync + 'static,
{
    pub program: Arc<Mutex<MediaDataTracker<R>>>,
    _phantom: PhantomData<R>,
}

impl<R> UnitDataStream<R>
where
    R: DbRepo + Clone + Send + Sync + 'static,
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
    R: DbRepo + Clone + Send + Sync + 'static,
{
    async fn transfer_media_data(
        &self,
        req: Request<MediaDataBatchResponse>,
    ) -> Result<Response<Empty>, Status> {
        //channel and compputeunit share a pv. so not use network to fetch data between channlle and computeunit.
        //if plan to support networks seperate, impl this
        todo!()
    }
    async fn transfer_tabular_data(
        &self,
        req: Request<TabularDataBatchResponse>,
    ) -> Result<Response<Empty>, tonic::Status> {
        todo!()
    }
}
