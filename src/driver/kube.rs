use super::{ChannelHandler, Driver, PipelineController, UnitHandler};
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
use std::sync::{Arc, Mutex};
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;
use tracing::debug;
pub struct KubeChannelHander
{
    pub deployment: Deployment,
    pub claim: PersistentVolumeClaim,
    pub service: Service,
}

impl Default for KubeChannelHander
{
    fn default() -> Self {
        Self {
            deployment: Default::default(),
            claim: PersistentVolumeClaim::default(),
            service: Default::default(),
        }
    }
}

impl ChannelHandler for KubeChannelHander
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

pub struct KubeHandler
{
    pub deployment: Deployment,
    pub claim: PersistentVolumeClaim,
    pub service: Service,
    pub channel: Option<KubeChannelHander>,
}

impl Default for KubeHandler
{
    fn default() -> Self {
        Self {
            deployment: Default::default(),
            claim: PersistentVolumeClaim::default(),
            service: Default::default(),
            channel: None,
        }
    }
}

impl UnitHandler for KubeHandler
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
    async fn channel_handler(&self) -> Result<Option<Arc<Mutex<KubeChannelHander>>>> {
        todo!()
    }
}

pub struct KubePipelineController
{
    handlers: HashMap<String, KubeHandler>,
}

impl<'a> Default for KubePipelineController
{
    fn default() -> Self {
        Self {
            handlers: Default::default(),
        }
    }
}

impl PipelineController for KubePipelineController
{
    async fn get_node<'a>(&'a self, id: &'a String) -> Result<&'a impl UnitHandler> {
        self.handlers.get(id).anyhow("id not found")
    }

    async fn get_node_mut<'a>(&'a mut self, id: &'a String) -> Result<&'a mut impl UnitHandler> {
        self.handlers.get_mut(id).anyhow("id not found")
    }
}

pub struct KubeDriver<'reg>
{
    reg: Handlebars<'reg>,
    client: Client,
}

impl<'reg> KubeDriver<'reg>
{
    pub async fn default() -> Result<KubeDriver<'reg>> {
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
            reg: reg,
            client: client,
        })
    }

    pub async fn from_k8s_client(client: Client) -> Result<KubeDriver<'reg>> {
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

impl Driver for KubeDriver<'_>
{
    #[allow(refining_impl_trait)]
    async fn deploy(&self, ns: &str, graph: &Dag) -> Result<KubePipelineController> {
        Self::ensure_namespace_exit_and_clean(&self.client, ns).await?;

        let deployment_api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        let claim_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), ns);
        let service_api: Api<Service> = Api::namespaced(self.client.clone(), ns);

        let mut pipeline_ctl = KubePipelineController::default();
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
                    claim: claim_deployment,
                    deployment: channel_deployment,
                    service: channel_service,
                })
            } else {
                None
            };

            let handler = KubeHandler {
                claim: claim_deployment,
                deployment: unit_deployment,
                service: unit_service,
                channel: channel_handler,
            };

            pipeline_ctl.handlers.insert(node.name.clone(), handler);
        }
        Ok(pipeline_ctl)
    }

    #[allow(refining_impl_trait)]
    async fn attach(
        &self,
        _namespace: &str,
        _graph: &Dag,
    ) -> Result<KubePipelineController> {
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

    #[tokio::test]
    async fn test_render() {
        env::set_var("RUST_LOG", "DEBUG");
        tracing_subscriber::fmt::init();

        let json_str = r#"
        {
          "name": "example",
          "version": "v1",
          "dag": [
            {
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
              "name": "computeunit2",
              "node_type": "ComputeUnit",
              "dependency": [
                "computeunit1"
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
        let dag = Dag::from_json(json_str).unwrap();
        let kube_driver = KubeDriver::default().await.unwrap();
        kube_driver.deploy("ntest", &dag).await.unwrap();
        //    kube_driver.clean("ntest").await.unwrap();
    }
}
