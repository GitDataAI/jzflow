use anyhow::{anyhow, Error, Result};
use jz_action::core::models::{DataRecord, DataState, DbRepo, Direction, TrackerState};
use jz_action::network::common::Empty;
use jz_action::network::datatransfer::data_stream_client::DataStreamClient;
use jz_action::network::datatransfer::MediaDataBatchResponse;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{self, sleep, Instant};
use tokio::{fs, select};
use tokio_stream::StreamExt;
use tonic::Status;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct ChannelTracker<R>
where
    R: DbRepo,
{
    pub(crate) repo: R,

    pub(crate) name: String,

    pub(crate) buf_size: usize,

    pub(crate) tmp_store: PathBuf,

    pub(crate) local_state: TrackerState,

    pub(crate) upstreams: Vec<String>,

    pub(crate) downstreams: Vec<String>,

    pub(crate) receiver_rx: Option<Sender<(MediaDataBatchResponse, oneshot::Sender<Result<()>>)>>,
}

impl<R> ChannelTracker<R>
where
    R: DbRepo,
{
    pub(crate) fn new(repo: R, name: &str, buf_size: usize, tmp_store: PathBuf) -> Self {
        ChannelTracker {
            name: name.to_string(),
            repo: repo,
            tmp_store,
            buf_size,
            local_state: TrackerState::Init,
            upstreams: vec![],
            downstreams: vec![],
            receiver_rx: None,
        }
    }

    pub(crate) async fn route_data(&mut self) -> Result<()> {
        if self.upstreams.len() == 0 {
            return Err(anyhow!("no upstream"));
        }

        let (tx, mut rx) = mpsc::channel(1);
        self.receiver_rx = Some(tx);

        let tmp_store = self.tmp_store.clone();
        let db_repo = self.repo.clone();
        let node_name = self.name.clone();
        let buf_size = self.buf_size;
        tokio::spawn(async move {
            loop {
                select! {
                 Some((data_batch, resp)) = rx.recv() => { //make this params
                    //save to fs
                    //create input directory
                    let now = Instant::now();
                    let id = data_batch.id;
                    //check limit
                   if let Err(err) =  db_repo.count_pending(&node_name,Direction::In).await.and_then(|count|{
                        if count > buf_size {
                            Err(anyhow!("has reach limit current:{count} limit:{buf_size}"))
                        } else {
                            Ok(())
                        }
                    }){
                        resp.send(Err(anyhow!("cannt query limit from mongo {err}"))).expect("request alread listen this channel");
                        continue;
                    }

                    // processed before
                    match db_repo.find_by_node_id(&node_name,&id,Direction::In).await  {
                        Ok(Some(_))=>{
                            warn!("data {} processed before", &id);
                            resp.send(Ok(())).expect("request alread listen this channel");
                            continue;
                        }
                        Err(err)=>{
                            error!("query mongo by id {err}");
                            resp.send(Err(err)).expect("request alread listen this channel");
                            continue;
                        }
                       _=>{}
                    }

                    // code below this can be move another coroutine

                    //write batch files
                    //write files is too slow. try to use mem cache
                    let tmp_in_path = tmp_store.join(id.clone());
                    debug!("try to create directory {:?} {:?}", tmp_in_path, now.elapsed());
                    if let Err(err) = fs::create_dir_all(&tmp_in_path).await {
                        error!("create input dir {:?} fail {}", tmp_in_path, err);
                        resp.send(Err(err.into())).expect("request alread listen this channel");
                        continue;
                    }

                    let mut is_write_err = false;
                    for (entry_path, entry) in  data_batch.cells.iter().map(|entry|(tmp_in_path.join(entry.path.clone()), entry)) {
                        if let Err(err) = fs::write(entry_path.clone(), &entry.data).await {
                            error!("write file {:?} fail {}", entry_path, err);
                            is_write_err = true;
                            break;
                        }
                    }
                    if is_write_err {
                        resp.send(Err(anyhow!("write file "))).expect("request alread listen this channel");
                        continue;
                    }
                    info!("write files to disk in {} {:?}", &id, now.elapsed());

                    //insert record to database
                    if let Err(err) = db_repo.insert_new_path(&DataRecord{
                        node_name: node_name.clone(),
                        id:id.clone(),
                        size: data_batch.size,
                        state: DataState::Received,
                        sent: vec![],
                        direction:Direction::In,
                    }).await{
                        error!("insert a databatch {err}");
                        resp.send(Err(err)).expect("request alread listen this channel");
                        continue;
                    }

                    resp.send(Ok(())).expect("request alread listen this channel ");
                    info!("insert a data batch in {} {:?}", &id, now.elapsed());
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
