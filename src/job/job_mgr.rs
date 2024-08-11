use crate::{
    core::db::{
        Job,
        JobDbRepo,
        JobState,
        JobUpdateInfo,
        ListJobParams,
        MainDbRepo,
    },
    dag::Dag,
    dbrepo::MongoRunDbRepo,
    driver::{
        ChannelHandler, Driver, NodeStatus, PipelineController, UnitHandler
    },
    utils::{
        IntoAnyhowResult,
        StdIntoAnyhowResult,
    },
};
use anyhow::Result;
use futures::future::try_join_all;
use kube::Client;
use mongodb::bson::oid::ObjectId;
use serde::{
    Deserialize,
    Serialize,
};
use std::marker::PhantomData;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
};

#[derive(Serialize, Deserialize)]
pub struct JobDetails {
    pub job: Job,
    pub node_status: Option<Vec<NodeStatus>>,
}

#[derive(Clone)]
pub struct JobManager<D, MAINR, JOBR>
where
    D: Driver,
    JOBR: JobDbRepo,
    MAINR: MainDbRepo,
{
    driver: D,
    db: MAINR,
    connection_string: String,
    _phantom_data: PhantomData<JOBR>,
}

impl<D, MAINR, JOBR> JobManager<D, MAINR, JOBR>
where
    D: Driver,
    JOBR: JobDbRepo,
    MAINR: MainDbRepo,
{
    pub async fn new(
        _client: Client,
        connection_string: &str,
        driver: D,
        db: MAINR,
    ) -> Result<Self> {
        Ok(JobManager {
            db,
            driver,
            connection_string: connection_string.to_string(),
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

                if let Err(err) = {
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

                    let finish_jobs_params = &ListJobParams {
                        state: Some(JobState::Finish),
                    };

                    for job in db.list_jobs(finish_jobs_params).await? {
                        let namespace = format!("{}-{}", job.name, job.retry_number - 1);
                        driver.clean(&namespace).await?;
                        db.update(
                            &job.id,
                            &JobUpdateInfo {
                                state: Some(JobState::Clean),
                            },
                        )
                        .await?;
                    }
                    anyhow::Ok(())
                } {
                    error!("error in job backend {err}");
                }
            }
        });

        Ok(())
    }

    pub async fn get_job_details(&self, id: &ObjectId) -> Result<JobDetails> {
        let job = self.db.get(id).await?.anyhow("job not found")?;

        let node_status = if job.state == JobState::Running {
            let dag = Dag::from_json(job.graph_json.as_str())?;
            let namespace = format!("{}-{}", job.name, job.retry_number - 1);
            let controller = self.driver.attach(&namespace, &dag).await?;
            let nodes = controller.nodes_in_order().anyhow()?;
            let nodes_controller =
                try_join_all(nodes.iter().map(|node_name| controller.get_node(node_name))).await?;

            let mut node_status = vec![];
            for node_ctl in nodes_controller {
                if let Some(channel) = node_ctl.channel_handler() {
                    let status = channel.status().await?;
                    node_status.push(status);  
                }
                let status = node_ctl.status().await?;
                node_status.push(status);
            }
            Some(node_status)
        } else {
            None
        };

        Ok(JobDetails { job, node_status })
    }

    pub async fn start_job(&self, id: &ObjectId) -> Result<()> {
        let job = self.db.get(id).await?.anyhow("job not found")?;
        let dag = Dag::from_json(job.graph_json.as_str())?;
        let namespace = format!("{}-{}", job.name, job.retry_number - 1);
        let controller = self.driver.attach(&namespace, &dag).await?;
        controller.start().await
    }

    pub async fn clean_job(&self, id: &ObjectId) -> Result<()> {
        let job = self.db.get(id).await?.anyhow("job not found")?;
        let namespace = format!("{}-{}", job.name, job.retry_number - 1);

        //clean k8s
        self.db
            .update(
                id,
                &JobUpdateInfo {
                    state: Some(JobState::Finish),
                },
            )
            .await?;
        self.driver.clean(&namespace).await?;
        //drop database
        let db_url = self.connection_string.clone() + "/" + &namespace;
        MongoRunDbRepo::drop(&db_url).await?;
        self.db
            .update(
                id,
                &JobUpdateInfo {
                    state: Some(JobState::Clean),
                },
            )
            .await?;
        Ok(())
    }
}
