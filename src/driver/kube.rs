use super::{ChannelHandler, Driver, PipelineController, UnitHandler};
use crate::core::GID;
use crate::utils::IntoAnyhowResult;
use crate::Dag;
use anyhow::Result;
use handlebars::Handlebars;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use kube::api::PostParams;
use kube::{Api, Client};
use std::collections::HashMap;
use std::default::Default;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
pub struct KubeChannelHander<ID>
where
    ID: GID,
{
    _id: PhantomData<ID>,
    pub deployment: Deployment,
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
}

impl<'reg, ID> KubeDriver<'reg, ID>
where
    ID: GID,
{
    pub fn new() -> Result<KubeDriver<'reg, ID>> {
        let mut reg = Handlebars::new();
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
        })
    }
}

impl<ID> Driver<ID> for KubeDriver<'_, ID>
where
    ID: GID,
{
    #[allow(refining_impl_trait)]
    async fn deploy(&self, namespace: &str, graph: &Dag<ID>) -> Result<KubePipelineController<ID>> {
        let client: Client = Client::try_default().await?;
        let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
        let service_api: Api<Service> = Api::namespaced(client, namespace);

        let mut pipeline_ctl = KubePipelineController::<ID>::default();
        for node in graph.iter() {
            let unit_deployment: Deployment =
                serde_json::from_str(self.reg.render("deployment", node)?.as_str())?;
            let unit_deployment = deployment_api
                .create(&PostParams::default(), &unit_deployment)
                .await?;

            let unit_service: Service =
                serde_json::from_str(self.reg.render("service", node)?.as_str())?;
            let unit_service = service_api
                .create(&PostParams::default(), &unit_service)
                .await?;

            let channel_handler = if node.channel.is_some() {
                let channel_deployment: Deployment =
                    serde_json::from_str(self.reg.render("channel_deployment", node)?.as_str())?;
                let channel_deployment = deployment_api
                    .create(&PostParams::default(), &channel_deployment)
                    .await?;

                let channel_service: Service =
                    serde_json::from_str(self.reg.render("channel_service", node)?.as_str())?;
                let channel_service = service_api
                    .create(&PostParams::default(), &channel_service)
                    .await?;
                Some(KubeChannelHander {
                    _id: std::marker::PhantomData,
                    deployment: channel_deployment,
                    service: channel_service,
                })
            } else {
                None
            };

            let handler = KubeHandler {
                _id: std::marker::PhantomData,
                deployment: unit_deployment,
                service: unit_service,
                channel: channel_handler,
            };

            pipeline_ctl.handlers.insert(node.id, handler);
        }
        Ok(pipeline_ctl)
    }

    #[allow(refining_impl_trait)]
    async fn attach(&self, namespace: &str, graph: &Dag<ID>) -> Result<KubePipelineController<ID>> {
        todo!()
    }

    async fn clean(&self, namespace: &str) -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_render() {
        let json_str = r#"
        {
          "id":"5c42b900-a87f-45e3-ba06-c40d94ad5ba2",
          "name": "example",
          "version": "v1",
          "dag": [
            {
              "id": "5c42b900-a87f-45e3-ba06-c40d94ad5ba2",
              "name": "ComputeUnit1",
              "dependency": [
                
              ],
              "spec": {
                "cmd": [
                  "ls"
                ],
                "image": "",
                "replicas":1
              },
              "channel": {
                "spec": {
                  "cmd": [
                    "bufsize",
                    "1024"
                  ],
                  "image": "jiaozifs:",
                  "replicas":1
                }
              }
            },
            {
              "id": "353fc5bf-697e-4221-8487-6ab91915e2a1",
              "name": "ComputeUnit2",
              "node_type": "ComputeUnit",
              "dependency": [
                "5c42b900-a87f-45e3-ba06-c40d94ad5ba2"
              ],
              "spec": {
                "cmd": [
                  "ls"
                ],
                "replicas":1,
                "image": ""
              }
            }
          ]
        }
                        "#;
        let dag = Dag::<Uuid>::from_json(json_str).unwrap();
        let kube_driver = KubeDriver::<Uuid>::new().unwrap();
        kube_driver.deploy("namespace", &dag).await.unwrap();
    }
}
