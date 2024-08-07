use crate::{
    core::db::{
        JobDbRepo,
        MainDbRepo,
    },
    dag::Dag,
    driver::Driver,
};
use anyhow::Result;
use kube::Client;
use std::marker::PhantomData;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::error;
pub struct JobManager<D, MAINR, JOBR>
where
    D: Driver,
    JOBR: JobDbRepo,
    MAINR: MainDbRepo,
{
    driver: D,
    db: MAINR,
    _phantom_data: PhantomData<JOBR>,
}

impl<D, MAINR, JOBR> JobManager<D, MAINR, JOBR>
where
    D: Driver,
    JOBR: JobDbRepo,
    MAINR: MainDbRepo,
{
    pub async fn new(client: Client, driver: D, db: MAINR, db_url: &str) -> Result<Self> {
        Ok(JobManager {
            db,
            driver,
            _phantom_data: PhantomData,
        })
    }
}

impl<D, MAINR, JOBR> JobManager<D, MAINR, JOBR>
where
    D: Driver,
    JOBR: JobDbRepo,
    MAINR: MainDbRepo,
{
    pub async fn run_backend(&self, token: CancellationToken) -> Result<JoinSet<Result<()>>> {
        let mut join_set = JoinSet::new();
        {
            let db = self.db.clone();
            let driver = self.driver.clone();
            join_set.spawn(async move {
                loop {
                    if token.is_cancelled() {
                        return Ok(());
                    }

                    while let Some(job) = db.get_job_for_running().await? {
                        let dag = Dag::from_json(job.graph_json.as_str())?;
                        let namespace = format!("{}-{}", job.name, job.retry_number);
                        if let Err(err) = driver.deploy(namespace.as_str(), &dag).await {
                            error!("run job {} {err}", job.name);
                        };
                    }
                }
            });
        }

        Ok(join_set)
    }
}
