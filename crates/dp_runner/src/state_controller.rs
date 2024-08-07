use anyhow::Result;
use jz_action::core::db::{
    JobDbRepo,
    TrackerState,
};
use std::sync::Arc;
use tokio::{
    select,
    sync::Mutex,
    task::JoinSet,
    time::{
        self,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    error,
    info,
};

use crate::channel_tracker::ChannelTracker;

pub struct StateController<R>
where
    R: JobDbRepo,
{
    pub program: Arc<Mutex<ChannelTracker<R>>>,
}

impl<R> StateController<R>
where
    R: JobDbRepo,
{
    pub async fn apply_db_state(
        &self,
        token: CancellationToken,
        repo: R,
        name: &str,
    ) -> Result<()> {
        let mut interval = time::interval(time::Duration::from_secs(10));
        let mut join_set: Option<JoinSet<Result<()>>> = None;
        let program = self.program.clone();
        loop {
            select! {
                _ = token.cancelled() => {
                    if let Some(mut join_set) = join_set {
                        info!("wait for route data exit");
                        while let Some(Err(err)) = join_set.join_next().await {
                            error!("exit spawn {err}");
                        }
                        info!("route data exit gracefully");
                    }
                   return Ok(());
                }
                _ = interval.tick() => {
                    match repo
                    .get_node_by_name(&name)
                    .await{
                        Ok(record)=> {
                            debug!("{} fetch state from db", record.node_name);
                            match record.state {
                                TrackerState::Ready => {
                                    let mut program_guard = program.lock().await;
                                    let cloned_token = token.clone();
                                    if matches!(program_guard.local_state, TrackerState::Init) {
                                        //start
                                        info!("set to ready state {:?}", record.incoming_streams);
                                        program_guard.local_state = TrackerState::Ready;
                                        program_guard.upstreams = record.incoming_streams;
                                        program_guard.downstreams = record.outgoing_streams;
                                        join_set = Some(program_guard.route_data(cloned_token).await?);
                                    }
                                }
                                TrackerState::Stop => {
                                    todo!()
                                }
                                _ => {}
                            }
                        },
                        Err(err)=> error!("fetch node state from db {err}")
                    }

                }
            }
        }
    }
}
