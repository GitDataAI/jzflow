use anyhow::{anyhow, Ok, Result};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_client::DataStreamClient;
use jz_action::network::datatransfer::DataBatchResponse;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_stream::{Stream, StreamExt, StreamMap};
use tonic::Status;
use tracing::{error, info};
use tracing_subscriber::registry::Data;

use crate::mprc;

#[derive(Debug)]
pub(crate) enum ProgramState {
    Init,
    Ready,
    Pending,
    Finish,
    Stopped,
}

pub(crate) struct BatchProgram {
    pub(crate) state: ProgramState,

    pub(crate) upstreams: Option<Vec<String>>,

    pub(crate) script: Option<String>,

    pub receivers: Arc<Mutex<mprc::Mprs<String, mpsc::Sender<Result<DataBatchResponse, Status>>>>>,
}

impl BatchProgram {
    pub(crate) fn new() -> Self {
        BatchProgram {
            state: ProgramState::Init,
            upstreams: None,
            script: None,
            receivers: Arc::new(Mutex::new(mprc::Mprs::new())),
        }
    }

    pub(crate) async fn fetch_data(&self) -> Result<()> {
        if self.upstreams.is_none() {
            return Err(anyhow!("no upstream"));
        }

        let (tx, mut rx) = mpsc::channel(1024);

        for upstream in self.upstreams.as_ref().unwrap() {
            let upstream_clone = upstream.clone();
            let tx_clone = tx.clone();
            let _ = tokio::spawn(async move {
                let mut client = DataStreamClient::connect(upstream_clone).await?;
                let mut stream = client.subscribe_new_data(Empty {}).await?.into_inner();

                while let Some(item) = stream.next().await {
                    tx_clone.send(item.unwrap()).await.unwrap();
                }

                error!("unable read data from stream");
                Ok(())
            });

            info!("listen data from upstream {}", upstream);
        }

        let receivers = self.receivers.clone();
        tokio::spawn(async move {
            loop {
                select! {
                 data_batch = rx.recv() => {
                    let sender = {
                         let mut guard = receivers.lock().await;
                         guard.get_random().unwrap().clone()
                    };

                    if let Err(e) = sender.send(std::result::Result::Ok(data_batch.unwrap())).await{
                        error!("unable send data to downstream {e}");
                    }
                 },
                }
            }
        });
        Ok(())
    }
}
