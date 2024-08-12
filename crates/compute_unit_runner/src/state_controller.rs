use actix_web::dev::ServerHandle;
use anyhow::Result;
use jiaoziflow::core::db::{
    JobDbRepo,
    TrackerState,
};
use std::sync::Arc;
use tokio::{
    select,
    sync::RwLock,
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

use crate::data_tracker::MediaDataTracker;

pub struct StateController<R>
where
    R: JobDbRepo,
{
    pub program: Arc<RwLock<MediaDataTracker<R>>>,
    pub _handler: ServerHandle,
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
                    .get_node_by_name(name)
                    .await{
                        Ok(record)=> {
                            debug!("{} fetch state from db", record.node_name);
                            let mut program_guard = program.write().await;
                            let mut local_state = program_guard.local_state.write().await;
                            if *local_state == record.state {
                                continue
                            }
                            let old_local_state = local_state.clone();
                            *local_state = record.state.clone();
                            info!("update state {:?} -> {:?}", old_local_state, local_state);
                            drop(local_state);
                            if  record.state!= TrackerState::Init && !record.state.is_end_state() {
                                //start
                                info!("start data processing");
                                join_set = Some(program_guard.route_data(token.clone()).await?);
                            }
                        },
                        Err(err)=> error!("fetch node state from db {err}")
                    }

                }
            }
        }
    }
}
