use super::{ChannelHandler, Driver, PipelineController, UnitHandler};
use crate::core::GID;
use crate::dag::Dag;
use crate::utils::IntoAnyhowResult;
use anyhow::{anyhow, Result};
use handlebars::Handlebars;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Namespace, PersistentVolumeClaim, Service};
use kube::api::{DeleteParams, PostParams};
use kube::{Api, Client};
use serde::Serialize;
use std::collections::HashMap;
use std::default::Default;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;
use tracing::debug;
pub struct KubeChannelHander<ID>
where
    ID: GID,
{
    _id: PhantomData<ID>,
    pub deployment: Deployment,
    pub claim: PersistentVolumeClaim,
    pub service: Service,
}

impl<ID> Default for KubeChannelHander<ID>
where
    ID: GID,
{
    fn default() -> Self {
        Self {
            _id: PhantomData,
            deployment: Default::default(),
            claim: PersistentVolumeClaim::default(),
            service: Default::default(),
        }
    }
}

impl<ID> ChannelHandler<ID> for KubeChannelHander<ID>
where
    ID: GID,
{
    async fn pause(&mut self) -> Result<()> {
        todo!()
    }

    async fn restart(&mut self) -> Result<()> {
        todo!()
    }

    async fn stop(&mut self) -> Result<()> {
        todo!()
    }
}

pub struct KubeHandler<ID>
where
    ID: GID,
{
    _id: PhantomData<ID>,
    pub deployment: Deployment,
    pub claim: PersistentVolumeClaim,
    pub service: Service,
    pub channel: Option<KubeChannelHander<ID>>,
}

impl<ID> Default for KubeHandler<ID>
where
    ID: GID,
{
    fn default() -> Self {
        Self {
            _id: PhantomData,
            deployment: Default::default(),
            claim: PersistentVolumeClaim::default(),
            service: Default::default(),
            channel: None,
        }
    }
}

impl<ID> UnitHandler<ID> for KubeHandler<ID>
where
    ID: GID,
{
    async fn pause(&mut self) -> Result<()> {
        todo!()
    }

    async fn restart(&mut self) -> Result<()> {
        todo!()
    }

    async fn stop(&mut self) -> Result<()> {
        todo!()
    }

    #[allow(refining_impl_trait)]
    async fn channel_handler(&self) -> Result<Option<Arc<Mutex<KubeChannelHander<ID>>>>> {
        todo!()
    }
}

pub struct KubePipelineController<ID>
where
    ID: GID,
{
    _id: PhantomData<ID>,
    handlers: HashMap<ID, KubeHandler<ID>>,
}

impl<'a, ID> Default for KubePipelineController<ID>
where
    ID: GID,
{
    fn default() -> Self {
        Self {
            _id: PhantomData,
            handlers: Default::default(),
        }
    }
}

impl<ID> PipelineController<ID> for KubePipelineController<ID>
where
    ID: GID,
{
    async fn get_node<'a>(&'a self, id: &'a ID) -> Result<&'a impl UnitHandler<ID>> {
        self.handlers.get(id).anyhow("id not found")
    }

    async fn get_node_mut<'a>(&'a mut self, id: &'a ID) -> Result<&'a mut impl UnitHandler<ID>> {
        self.handlers.get_mut(id).anyhow("id not found")
    }
}

pub struct KubeDriver<'reg, ID>
where
    ID: GID,
{
    _id: std::marker::PhantomData<ID>,
    reg: Handlebars<'reg>,
    client: Client,
}

impl<'reg, ID> KubeDriver<'reg, ID>
where
    ID: GID,
{
    pub async fn default() -> Result<KubeDriver<'reg, ID>> {
        let mut reg = Handlebars::new();
        reg.register_template_string("claim", include_str!("kubetpl/claim.tpl"))?;

        reg.register_template_string("deployment", include_str!("kubetpl/deployment.tpl"))?;
        reg.register_template_string("service", include_str!("kubetpl/service.tpl"))?;
        reg.register_template_string(
            "channel_deployment",
            include_str!("kubetpl/channel_deployment.tpl"),
        )?;
        reg.register_template_string(
            "channel_service",
            include_str!("kubetpl/channel_service.tpl"),
        )?;

        let client = Client::try_default().await?;
        Ok(KubeDriver {
            _id: std::marker::PhantomData,
            reg: reg,
            client: client,
        })
    }

    pub async fn from_k8s_client(client: Client) -> Result<KubeDriver<'reg, ID>> {
        let mut reg = Handlebars::new();
        reg.register_template_string("claim", include_str!("kubetpl/claim.tpl"))?;

        reg.register_template_string("deployment", include_str!("kubetpl/deployment.tpl"))?;
        reg.register_template_string("service", include_str!("kubetpl/service.tpl"))?;
        reg.register_template_string(
            "channel_deployment",
            include_str!("kubetpl/channel_deployment.tpl"),
        )?;
        reg.register_template_string(
            "channel_service",
            include_str!("kubetpl/channel_service.tpl"),
        )?;
        Ok(KubeDriver {
            _id: std::marker::PhantomData,
            reg: reg,
            client: client,
        })
    }

    async fn ensure_namespace_exit_and_clean(client: &Client, ns: &str) -> Result<()> {
        let namespace = Namespace {
            metadata: kube::api::ObjectMeta {
                name: Some(ns.to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let namespaces: Api<Namespace> = Api::all(client.clone());
        // Create the namespace
        if namespaces.get(ns).await.is_ok() {
            let _ = namespaces
                .delete(ns, &DeleteParams::default())
                .await
                .map(|_| ())
                .map_err(|e| anyhow!("{}", e.to_string()));
            let retry_strategy = ExponentialBackoff::from_millis(1000).take(20);
            let _ = Retry::spawn(retry_strategy, || async {
                match namespaces.get(ns).await {
                    Ok(_) => Err(anyhow!("expect deleted")),
                    Err(e) => {
                        if e.to_string().contains("not found") {
                            Ok(())
                        } else {
                            Err(anyhow!("retry"))
                        }
                    }
                }
            })
            .await?;
        }
        namespaces
            .create(&PostParams::default(), &namespace)
            .await
            .map(|_| ())
            .map_err(|e| anyhow!("{}", e.to_string()))
    }
}

#[derive(Serialize)]
struct ClaimRenderParams {
    name: String,
}

impl<ID> Driver<ID> for KubeDriver<'_, ID>
where
    ID: GID,
{
    #[allow(refining_impl_trait)]
    async fn deploy(&self, ns: &str, graph: &Dag<ID>) -> Result<KubePipelineController<ID>> {
        Self::ensure_namespace_exit_and_clean(&self.client, ns).await?;

        let deployment_api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        let claim_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), ns);
        let service_api: Api<Service> = Api::namespaced(self.client.clone(), ns);

        let mut pipeline_ctl = KubePipelineController::<ID>::default();
        for node in graph.iter() {
            let claim_string = self.reg.render(
                "claim",
                &ClaimRenderParams {
                    name: node.name.clone() + "-node-claim",
                },
            )?;
            debug!("rendered clam string {}", claim_string);
            let claim: PersistentVolumeClaim = serde_json::from_str(&claim_string)?;
            let claim_deployment = claim_api.create(&PostParams::default(), &claim).await?;

            let deployment_string = self.reg.render("deployment", node)?;
            debug!("rendered unit clam string {}", deployment_string);

            let unit_deployment: Deployment = serde_json::from_str(&deployment_string)?;
            let unit_deployment = deployment_api
                .create(&PostParams::default(), &unit_deployment)
                .await?;

            let service_string = self.reg.render("service", node)?;
            debug!("rendered unit service config {}", service_string);

            let unit_service: Service = serde_json::from_str(service_string.as_str())?;
            let unit_service = service_api
                .create(&PostParams::default(), &unit_service)
                .await?;

            let channel_handler = if node.channel.is_some() {
                let claim_string = self.reg.render(
                    "claim",
                    &ClaimRenderParams {
                        name: node.name.clone() + "-channel-claim",
                    },
                )?;
                debug!("rendered channel claim string {}", claim_string);
                let claim: PersistentVolumeClaim = serde_json::from_str(&claim_string)?;
                let claim_deployment = claim_api.create(&PostParams::default(), &claim).await?;

                let tmp = self.reg.get_template("channel_deployment").unwrap();
                let channel_deployment_string = self.reg.render("channel_deployment", node)?;
                debug!(
                    "rendered channel deployment string {}",
                    channel_deployment_string
                );
                let channel_deployment: Deployment =
                    serde_json::from_str(channel_deployment_string.as_str())?;
                let channel_deployment = deployment_api
                    .create(&PostParams::default(), &channel_deployment)
                    .await?;

                let channel_service_string = self.reg.render("channel_service", node)?;
                debug!("rendered channel service string {}", deployment_string);

                let channel_service: Service =
                    serde_json::from_str(channel_service_string.as_str())?;
                let channel_service = service_api
                    .create(&PostParams::default(), &channel_service)
                    .await?;
                Some(KubeChannelHander {
                    _id: std::marker::PhantomData,
                    claim: claim_deployment,
                    deployment: channel_deployment,
                    service: channel_service,
                })
            } else {
                None
            };

            let handler = KubeHandler {
                _id: std::marker::PhantomData,
                claim: claim_deployment,
                deployment: unit_deployment,
                service: unit_service,
                channel: channel_handler,
            };

            pipeline_ctl.handlers.insert(node.id, handler);
        }
        Ok(pipeline_ctl)
    }

    #[allow(refining_impl_trait)]
    async fn attach(
        &self,
        _namespace: &str,
        _graph: &Dag<ID>,
    ) -> Result<KubePipelineController<ID>> {
        todo!()
    }

    async fn clean(&self, ns: &str) -> Result<()> {
        let client: Client = Client::try_default().await?;
        let namespaces: Api<Namespace> = Api::all(client.clone());
        if namespaces.get(ns).await.is_ok() {
            let _ = namespaces
                .delete(ns, &DeleteParams::default())
                .await
                .map(|_| ())
                .map_err(|e| anyhow!("{}", e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;
    use tracing_subscriber;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_render() {
        env::set_var("RUST_LOG", "DEBUG");
        tracing_subscriber::fmt::init();

        let json_str = r#"
        {
          "id":"5c42b900-a87f-45e3-ba06-c40d94ad5ba2",
          "name": "example",
          "version": "v1",
          "dag": [
            {
              "id": "5c42b900-a87f-45e3-ba06-c40d94ad5ba2",
              "name": "computeunit1",
              "dependency": [
                
              ],
              "spec": {
                "cmd": [
                  "ls"
                ],
                "image": "ubuntu:22.04",
                "replicas":1
              },
              "channel": {
                "spec": {
                  "cmd": [
                    "bufsize",
                    "1024"
                  ],
                  "image": "ubuntu:22.04",
                  "replicas":1
                }
              }
            },
            {
              "id": "353fc5bf-697e-4221-8487-6ab91915e2a1",
              "name": "computeunit2",
              "node_type": "ComputeUnit",
              "dependency": [
                "5c42b900-a87f-45e3-ba06-c40d94ad5ba2"
              ],
              "spec": {
                "cmd": [
                  "ls"
                ],
                "replicas":1,
                "image": "ubuntu:22.04"
              }
            }
          ]
        }
                        "#;
        let dag = Dag::<Uuid>::from_json(json_str).unwrap();
        let kube_driver = KubeDriver::<Uuid>::default().await.unwrap();
        kube_driver.deploy("ntest", &dag).await.unwrap();
        //    kube_driver.clean("ntest").await.unwrap();
    }
}
