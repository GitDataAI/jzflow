use crate::{
    core::db::{
        Job,
        JobDbRepo,
        JobState,
        JobUpdateInfo,
        ListJobParams,
        MainDbRepo, NodeRepo,
    },
    dag::Dag,
    dbrepo::MongoRunDbRepo,
    driver::{
        ChannelHandler,
        Driver,
        NodeStatus,
        PipelineController,
        UnitHandler,
    },
    utils::{
        IntoAnyhowResult,
        StdIntoAnyhowResult,
    },
};
use anyhow::{anyhow, Result};
use futures::future::try_join_all;
use kube::Client;
use mongodb::bson::oid::ObjectId;
use serde::{
    Deserialize,
    Serialize,
};
use std::{marker::PhantomData, time::Duration};
use tokio::{task::JoinSet, time::sleep};
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
        let connect_string = self.connection_string.clone();

        join_set.spawn(async move {
            info!("backend thead is running");
            loop {
                if token.is_cancelled() {
                    return Ok(());
                }

                if let Err(err) = {
                    //fetch data to deploy
                    {
                        while let Some(job) = db.get_job_for_running().await? {
                            let dag = Dag::from_json(job.graph_json.as_str())?;
                            let namespace = format!("{}-{}", job.name, job.retry_number);
                            match driver.deploy(namespace.as_str(), &dag).await {
                                Ok(_) => {
                                    if let Err(err) = db
                                        .update(
                                            &job.id,
                                            &JobUpdateInfo {
                                                state: Some(JobState::Deployed),
                                            },
                                        )
                                        .await
                                    {
                                        error!("set job to deploy state {err}");
                                    }
                                }
                                Err(err) => {
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
                                }
                            };
                        }
                    }
                    //mark finish if all nodes was finish
                    {
                        let running_jobs_params = &ListJobParams {
                            state: Some(JobState::Running),
                        };
                        for job in db.list_jobs(running_jobs_params).await? {
                            let namespace = format!("{}-{}", job.name, job.retry_number - 1);
                            let db_url = connect_string.clone() + "/" + &namespace;
                            let job_db =  MongoRunDbRepo::new(&db_url).await?;
                            job_db.is_all_node_finish().await.map(|_|{
                                db.update(
                                    &job.id,
                                    &JobUpdateInfo {
                                        state: Some(JobState::Finish),
                                    },
                                )
                            })?.await?;
                        }
                    }

                    //clean data
                    {
                        let finish_jobs_params = &ListJobParams {
                            state: Some(JobState::Finish),
                        };

                        for job in db.list_jobs(finish_jobs_params).await? {
                            let namespace = format!("{}-{}", job.name, job.retry_number - 1);
                            driver.clean(&namespace).await?;
                            let db_url = connect_string.clone() + "/" + &namespace;
                            MongoRunDbRepo::drop(&db_url).await?;
                            db.update(
                                &job.id,
                                &JobUpdateInfo {
                                    state: Some(JobState::Clean),
                                },
                            )
                            .await?;
                        }
                    }


                    anyhow::Ok(())
                } {
                    error!("error in job backend {err}");
                }

                sleep(Duration::from_secs(5)).await;
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
        if job.state != JobState::Deployed {
            return Err(anyhow!("only can run a deployed job"));
        }

        let dag = Dag::from_json(job.graph_json.as_str())?;
        let namespace = format!("{}-{}", job.name, job.retry_number - 1);
        let controller = self.driver.attach(&namespace, &dag).await?;
        controller.start().await?;
       self.db
        .update(
            &job.id,
            &JobUpdateInfo {
                state: Some(JobState::Running),
            },
        ).await
        .map_err(|err|{
            error!("set job to deploy state {err}");
            err
        })
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
