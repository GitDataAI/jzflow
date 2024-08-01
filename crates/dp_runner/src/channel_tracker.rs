use anyhow::{anyhow, Result};
use jz_action::core::models::{DataRecord, DataState, DbRepo, Direction, TrackerState};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_client::DataStreamClient;
use jz_action::network::datatransfer::MediaDataBatchResponse;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{self, sleep};
use tokio::{fs, select};
use tokio_stream::StreamExt;
use tonic::Status;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::mprc;
pub struct ChannelTracker<R>
where
    R: DbRepo,
{
    pub(crate) repo: R,

    pub(crate) name: String,

    pub(crate) tmp_store: PathBuf,

    pub(crate) local_state: TrackerState,

    pub(crate) upstreams: Vec<String>,

    pub(crate) downstreams: Vec<String>,

    pub receivers:
        Arc<Mutex<mprc::Mprs<String, mpsc::Sender<Result<MediaDataBatchResponse, Status>>>>>,
}

impl<R> ChannelTracker<R>
where
    R: DbRepo,
{
    pub(crate) fn new(repo: R, name: &str, tmp_store: PathBuf) -> Self {
        ChannelTracker {
            name: name.to_string(),
            repo: repo,
            tmp_store,
            local_state: TrackerState::Init,
            upstreams: vec![],
            downstreams: vec![],
            receivers: Arc::new(Mutex::new(mprc::Mprs::new())),
        }
    }

    pub(crate) async fn route_data(&self) -> Result<()> {
        if self.upstreams.len() == 0 {
            return Err(anyhow!("no upstream"));
        }

        let (tx, mut rx) = mpsc::channel(1);
        for upstream in &self.upstreams {
            let upstream = upstream.clone();
            let tx_clone = tx.clone();
            let _ = tokio::spawn(async move {
                loop {
                    //todo handle network disconnect
                    match DataStreamClient::connect(upstream.clone()).await {
                        Ok(mut client) => {
                            match client.subscribe_media_data(Empty {}).await {
                                Ok(stream) => {
                                    let mut stream = stream.into_inner();
                                    info!("start to listen new data from upstream {}", upstream);
                                    while let Some(resp) = stream.next().await {
                                        //TODO need to confirm item why can be ERR
                                        match resp {
                                            Ok(resp) => tx_clone.send(resp).await.unwrap(),
                                            Err(err) => {
                                                error!("receive a error from stream {err}");
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!("subscribe_media_data fail {} {err}", upstream.clone())
                                }
                            }
                        }
                        Err(err) => error!("connect upstream fail {} {err}", upstream.clone()),
                    }

                    error!("unable read data from stream, reconnect in 2s");
                    sleep(Duration::from_secs(2)).await;
                }
            });
        }

        let tmp_store = self.tmp_store.clone();
        let db_repo = self.repo.clone();
        let node_name = self.name.clone();
        tokio::spawn(async move {
            loop {
                select! {
                 Some(data_batch) = rx.recv() => {
                    //save to fs
                    //create input directory
                    let id = Uuid::new_v4().to_string();
                    let tmp_in_path = tmp_store.join(id.clone());

                    debug!("try to create directory {:?}", tmp_in_path);
                    if let Err(e) = fs::create_dir_all(&tmp_in_path).await {
                        error!("create input dir {:?} fail {}", tmp_in_path, e);
                        return
                    }

                    //write batch files
                    for entry in  data_batch.cells.iter() {
                        let entry_path = tmp_in_path.join(entry.path.clone());
                        if let Err(e) = fs::write(entry_path.clone(), &entry.data).await {
                            error!("write file {:?} fail {}", entry_path, e);
                        }
                    }

                    info!("insert a data batch in {}", &id);
                    //insert record to database
                    db_repo.insert_new_path(&DataRecord{
                        node_name: node_name.clone(),
                        id:id.clone(),
                        size: data_batch.size,
                        state: DataState::Received,
                        direction:Direction::In,
                    }).await.unwrap();
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
                .get_node_by_name(&(name.to_owned() + "-channel"))
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
                        program_guard.upstreams = record.upstreams;
                        program_guard.downstreams = record.downstreams;
                        program_guard.route_data().await?;
                    }
                }
                TrackerState::Stop => {
                    todo!()
                }
                _ => {}
            }
        }
    }
}
