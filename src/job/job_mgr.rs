use crate::{
    core::db::{
        DBConfig,
        JobDbRepo,
    },
    driver::kube::KubeDriver,
};
use anyhow::Result;
use kube::Client;
use serde::Serialize;

pub struct JobManager<'reg, JOBR, DBC>
where
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    driver: KubeDriver<'reg, JOBR, DBC>,
}

impl<'reg, JOBR, DBC> JobManager<'reg, JOBR, DBC>
where
    JOBR: JobDbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig,
{
    pub async fn new(client: Client, db_config: DBC) -> Result<JobManager<'reg, R, DBC>> {
        let driver = KubeDriver::new(client, db_config).await?;
        Ok(JobManager { driver })
    }
}
