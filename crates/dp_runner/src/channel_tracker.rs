use anyhow::{anyhow, Ok, Result};
use jz_action::core::models::{NodeRepo, TrackerState};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_client::DataStreamClient;
use jz_action::network::datatransfer::MediaDataBatchResponse;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, Mutex};
use tokio::time;
use tokio_stream::StreamExt;
use tonic::Status;
use tracing::{error, info};

use crate::mprc;
pub struct ChannelTracker<R>
where
    R: NodeRepo,
{
    pub(crate) _repo: R,
    pub(crate) _name: String,
    pub(crate) local_state: TrackerState,
    pub(crate) upstreams: Option<Vec<String>>,
    pub receivers:
        Arc<Mutex<mprc::Mprs<String, mpsc::Sender<Result<MediaDataBatchResponse, Status>>>>>,
}

impl<R> ChannelTracker<R>
where
    R: NodeRepo,
{
    pub(crate) fn new(repo: R, name: &str) -> Self {
        ChannelTracker {
            _name: name.to_string(),
            _repo: repo,
            local_state: TrackerState::Init,
            upstreams: None,
            receivers: Arc::new(Mutex::new(mprc::Mprs::new())),
        }
    }

    pub(crate) async fn route_data(&self) -> Result<()> {
        if self.upstreams.is_none() {
            return Err(anyhow!("no upstream"));
        }

        let (tx, mut rx) = mpsc::channel(1024);
        for upstream in self.upstreams.as_ref().unwrap() {
            let upstream_clone = upstream.clone();
            let tx_clone = tx.clone();
            let _ = tokio::spawn(async move {
                let mut client = DataStreamClient::connect(upstream_clone).await?;
                let mut stream = client.subscribe_media_data(Empty {}).await?.into_inner();

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

    pub async fn apply_db_state(
        repo: R,
        name: &str,
        program: Arc<Mutex<ChannelTracker<R>>>,
    ) -> Result<()> {
        let mut interval = time::interval(time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let record = repo
                .get_node_by_name(name)
                .await
                .expect("record has inserted in controller or network error");
            match record.state {
                TrackerState::Ready => {
                    let mut program_guard = program.lock().await;

                    if matches!(program_guard.local_state, TrackerState::Init) {
                        //start
                        program_guard.local_state = TrackerState::Ready;
                        program_guard.upstreams = Some(record.upstreams);
                        program_guard.route_data().await?;
                    }
                }
                TrackerState::Stop => {
                    todo!()
                }
                _ => {}
            }
        }
        Ok(())
    }
}
