use crate::{
    core::db::{
        Job,
        JobDbRepo,
        JobState,
        JobUpdateInfo,
        MainDbRepo,
        Node,
    },
    dag::Dag,
    driver::Driver,
    utils::IntoAnyhowResult,
};
use anyhow::Result;
use kube::Client;
use mongodb::bson::oid::ObjectId;
use std::marker::PhantomData;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
};
pub struct JobDetails {
    job: Job,
}

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
    pub fn run_backend(
        &self,
        join_set: &mut JoinSet<Result<()>>,
        token: CancellationToken,
    ) -> Result<()> {
        let db = self.db.clone();
        let driver = self.driver.clone();
        join_set.spawn(async move {
            info!("backend thead is running");
            loop {
                if token.is_cancelled() {
                    return Ok(());
                }

                while let Some(job) = db.get_job_for_running().await? {
                    let dag = Dag::from_json(job.graph_json.as_str())?;
                    let namespace = format!("{}-{}", job.name, job.retry_number);
                    if let Err(err) = driver.deploy(namespace.as_str(), &dag).await {
                        error!("run job {} {err}, start cleaning", job.name);
                        if let Err(err) = driver.clean(namespace.as_str()).await {
                            error!("clean job resource {err}");
                        }
                        if let Err(err) = db
                            .update(
                                &job.id,
                                &JobUpdateInfo {
                                    state: Some(JobState::Error),
                                },
                            )
                            .await
                        {
                            error!("set job to error state {err}");
                        }
                    };
                }
            }
        });

        Ok(())
    }

    pub async fn get_job_details(&self, id: &ObjectId) -> Result<JobDetails> {
        let job = self.db.get(id).await?.anyhow("job not found")?;
        let dag = Dag::from_json(job.graph_json.as_str())?;
        let namespace = format!("{}-{}", job.name, job.retry_number - 1);
        let controller = self.driver.attach(&namespace, &dag).await?;
        Ok(JobDetails { job: job })
    }
}
