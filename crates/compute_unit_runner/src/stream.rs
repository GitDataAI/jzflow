use crate::data_tracker::MediaDataTracker;
use anyhow::Result;
use jiaoziflow::core::db::Repo;
use jiaoziflow::network::{
    common::Empty,
    datatransfer::{
        data_stream_server::DataStream,
        DataBatch,
    },
};
use std::sync::Arc;
use tokio::sync::{
    oneshot,
    RwLock,
};
use tonic::{
    Request,
    Response,
    Status,
};

pub struct ChannelDataStream<R>
where
    R: Repo,
{
    pub program: Arc<RwLock<MediaDataTracker<R>>>,
}

#[tonic::async_trait]
impl<R> DataStream for ChannelDataStream<R>
where
    R: Repo,
{
    async fn transfer_media_data(
        &self,
        req: Request<DataBatch>,
    ) -> Result<Response<Empty>, Status> {
        let send_tx = {
            let program = self.program.read().await;
            if program.incoming_tx.is_none() {
                return Err(Status::internal("not ready"));
            }
            program.incoming_tx.as_ref().unwrap().clone()
        };

        let (tx, rx) = oneshot::channel::<Result<()>>();
        let req: DataBatch = req.into_inner();
        if let Err(err) = send_tx.send((req, tx)).await {
            return Err(Status::from_error(Box::new(err)));
        }

        match rx.await {
            Ok(Ok(_)) => Ok(Response::new(Empty {})),
            Ok(Err(err)) => Err(Status::internal(err.to_string())),
            Err(err) => Err(Status::from_error(Box::new(err))),
        }
    }

    async fn request_media_data(&self, _: Request<Empty>) -> Result<Response<DataBatch>, Status> {
        todo!();
    }
}
