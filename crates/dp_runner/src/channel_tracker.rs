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
use tracing::{debug, error, info};

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
            let upstream = upstream.clone();
            let tx_clone = tx.clone();
            let _ = tokio::spawn(async move {
                info!("connect upstream {}", upstream.clone());
                let mut client = DataStreamClient::connect(upstream.clone()).await?;
                info!("subscribe {}", upstream.clone());
                let mut stream = client.subscribe_media_data(Empty {}).await?.into_inner();
                info!("start to listen new data from upstream {}", upstream.clone());
                while let Some(resp) = stream.next().await {
                    debug!("receive a data and send to channel");
                    //TODO need to confirm item why can be ERR
                    match resp {
                        std::result::Result::Ok(resp) => tx_clone.send(resp).await.unwrap(),
                        Err(err) => error!("receive a error from stream {err}"),
                    }
                }

                error!("unable read data from stream");
                Ok(())
            });
        }

        let receivers = self.receivers.clone();
        tokio::spawn(async move {
            loop {
                select! {
                 data_batch = rx.recv() => {
                    if let Some(data_batch) = data_batch {
                        let (dest, sender) = {
                            let mut guard = receivers.lock().await;
                            if guard.count() == 0 {
                                debug!("receive a data but no sendable nodes");
                                continue;
                            }
                            guard.get_random().unwrap().clone()
                       };

                       debug!("select destination {dest}, and send data to this node");
                       if let Err(e) = sender.send(std::result::Result::Ok(data_batch)).await{
                           error!("unable send data to downstream {e}");
                       }
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
                .get_node_by_name(&(name.to_owned()))
                .await
                .expect("record has inserted in controller or network error");
            debug!("{} fetch state from db", record.node_name);
            match record.state {
                TrackerState::Ready => {
                    let mut program_guard = program.lock().await;

                    if matches!(program_guard.local_state, TrackerState::Init) {
                        info!("set to ready state {:?}", record.upstreams);
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
