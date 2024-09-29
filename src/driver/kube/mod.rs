use super::{NodeStatus, PodStauts, UnitHandler};
use crate::{
    core::db::Repo,
    utils::{
        k8s_helper::get_pod_status,
        StdIntoAnyhowResult,
    },
};
use anyhow::Result;
use k8s_metrics::v1beta1 as metricsv1;
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::PersistentVolumeClaim,
};
use kube::{
    api::ListParams,
    Api,
    Client,
};
use std::{
    collections::HashMap,
    default::Default,
};
use tracing::{
    error,
    warn,
};

mod pause;
mod restart;
mod start;
mod status;
mod stop;

pub struct KubeHandler<R>
where
    R: Repo,
{
    pub(crate) client: Client,
    pub(crate) node_name: String,
    pub(crate) namespace: String,
    pub(crate) stateset_name: String,
    pub(crate) claim_name: String,
    pub(crate) service_name: String,
    pub(crate) db_repo: R,
}

impl<R> KubeHandler<R>
where
    R: Repo,
{
    pub fn node_name(&self) -> String {
        self.node_name.clone()
    }
    pub fn namespace(&self) -> String {
        self.namespace.clone()
    }
    pub fn stateset_name(&self) -> String {
        self.stateset_name.clone()
    }
    pub fn claim_name(&self) -> String {
        self.claim_name.clone()
    }
    pub fn service_name(&self) -> String {
        self.service_name.clone()
    }
}

impl<T> UnitHandler for KubeHandler<T>
where T: Repo
{
    fn name(&self) -> String {
        self.node_name.clone()
    }

    async fn start(&self) -> Result<()>{
        self.inner_start().await?;
        Ok(())
    }

    async fn status(&self) -> Result<NodeStatus> {
        self.inner_status().await.anyhow()
    }

    async fn pause(&mut self) -> Result<()> {
            self.inner_pause().await?;
            Ok(())
    }

    async fn restart(&mut self) -> Result<()> {
            self.inner_restart().await?;
            Ok(())
    }

    async fn stop(&mut self) -> Result<()>{
        self.inner_stop().await?;
        Ok(())
    }
}