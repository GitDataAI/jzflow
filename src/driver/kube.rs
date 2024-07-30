use super::{ChannelHandler, Driver, PipelineController, UnitHandler};
use crate::core::models::{DBConfig, Graph, GraphRepo, Node, NodeRepo, NodeType, TrackerState};
use crate::core::{models::DbRepo, ComputeUnit};
use crate::dag::Dag;
use crate::dbrepo::mongo::MongoRepo;
use crate::utils::IntoAnyhowResult;
use anyhow::{anyhow, Result};
use handlebars::Handlebars;
use k8s_openapi::api::apps::v1:: StatefulSet;
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
use chrono::prelude::*;

pub struct KubeChannelHander<R>
where
    R: DbRepo,
{
    pub statefulset: StatefulSet,
    pub claim: PersistentVolumeClaim,
    pub service: Service,
    pub db_repo: R,
}

impl<R> KubeChannelHander<R>
where
    R: DbRepo,
{
    fn new(db_repo: R) -> Self {
        Self {
            statefulset: Default::default(),
            claim: PersistentVolumeClaim::default(),
            service: Default::default(),
            db_repo:db_repo
        }
    }
}

impl<R> ChannelHandler for KubeChannelHander<R>
where
    R: DbRepo,
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

pub struct KubeHandler<R>
where
    R: DbRepo,
{
    pub statefulset: StatefulSet,
    pub claim: PersistentVolumeClaim,
    pub service: Service,
    pub db_repo: R,
    pub channel: Option<KubeChannelHander<R>>,
}

impl<R> KubeHandler<R>
where
    R: DbRepo,
{
    fn new(repo: R) -> Self {
        Self {
            statefulset: Default::default(),
            claim: PersistentVolumeClaim::default(),
            service: Default::default(),
            channel: None,
            db_repo: repo,
        }
    }
}

impl<R> UnitHandler for KubeHandler<R>
where
    R: DbRepo,
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
    async fn channel_handler(&self) -> Result<Option<Arc<Mutex<KubeChannelHander<R>>>>> {
        todo!()
    }
}

pub struct KubePipelineController< R>
where
    R: DbRepo,
{
    pub db_repo: R,
    handlers: HashMap<String, KubeHandler< R>>,
}

impl<R> KubePipelineController< R>
where
    R: DbRepo,
{
    fn new(repo: R) -> Self {
        Self {
            db_repo: repo,
            handlers: Default::default(),
        }
    }
}

impl<R> PipelineController for KubePipelineController< R>
where
    R: DbRepo,
{
    async fn get_node<'b>(&'b self, id: &'b String) -> Result<&'b impl UnitHandler> {
        self.handlers.get(id).anyhow("id not found")
    }

    async fn get_node_mut<'b>(&'b mut self, id: &'b String) -> Result<&'b mut impl UnitHandler> {
        self.handlers.get_mut(id).anyhow("id not found")
    }
}

pub struct KubeDriver<'reg, R, DBC>
where
    R: DbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig+'static,
{
    reg: Handlebars<'reg>,
    client: Client,
    db_config: DBC,
    _phantom_data: PhantomData<R>
}

impl<'reg,  R, DBC> KubeDriver<'reg,  R, DBC>
where
    R: DbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig,
{
    pub async fn new(client: Client, db_config: DBC) -> Result<KubeDriver<'reg,  R, DBC>> {
        let mut reg = Handlebars::new();
        reg.register_template_string("claim", include_str!("kubetpl/claim.tpl"))?;

        reg.register_template_string("statefulset", include_str!("kubetpl/statefulset.tpl"))?;
        reg.register_template_string("service", include_str!("kubetpl/service.tpl"))?;
        reg.register_template_string(
            "channel_statefulset",
            include_str!("kubetpl/channel_statefulset.tpl"),
        )?;
        reg.register_template_string(
            "channel_service",
            include_str!("kubetpl/channel_service.tpl"),
        )?;
        Ok(KubeDriver {
            reg,
            client,
            db_config,
            _phantom_data: PhantomData,
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

#[derive(Serialize)]
struct NodeRenderParams<'a, DBC>
where
    DBC: Sized + Serialize + Send + Sync + DBConfig +'static,
{
    node: &'a ComputeUnit,
    log_level: &'a str,
    db: DBC,
}

impl<R, DBC> Driver for KubeDriver<'_,  R, DBC>
where
    R: DbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig+'static,
{
    #[allow(refining_impl_trait)]
    async fn deploy(&self, run_id: &str, graph: &Dag) -> Result<KubePipelineController<MongoRepo>>
    {
        Self::ensure_namespace_exit_and_clean(&self.client, run_id).await?;

        let repo = MongoRepo::new(self.db_config.clone(), run_id).await?;
        let statefulset_api: Api<StatefulSet> = Api::namespaced(self.client.clone(), run_id);
        let claim_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), run_id);
        let service_api: Api<Service> = Api::namespaced(self.client.clone(), run_id);

        // insert global record
        let cur_tm = Utc::now().timestamp();
        let graph_record = Graph{
            graph_json: graph.raw.clone(),
            created_at: cur_tm,
            updated_at: cur_tm,
        };
        repo.insert_global_state(graph_record).await?;

        let mut pipeline_ctl= KubePipelineController::new(repo.clone());
        for node in graph.iter() {
            let data_unit_render_args = NodeRenderParams {
                node,
                db: self.db_config.clone(),
                log_level: "debug",
            };

            // apply channel
            let channel_handler = if node.channel.is_some() {
                let upstreams = graph.get_incomming_nodes(&node.name).iter().map(|upstream|{
                    //<pod-name>.<service-name>.<namespace>.svc.cluster.local
                    format!("{}.{}.{}.svc.cluster.local",)
                });
                let channel_record = Node{
                    node_name: node.name.clone() +"-channel",
                    state: TrackerState::Init,
                    node_type: NodeType::Channel,
                    upstreams: upstreams.iter().map(|v|v.to_string()).collect(),
                    created_at: cur_tm,
                    updated_at: cur_tm,
                };

                repo.insert_node(channel_record).await?;

                let claim_string = self.reg.render(
                    "claim",
                    &ClaimRenderParams {
                        name: node.name.clone() + "-channel-claim",
                    },
                )?;
                debug!("rendered channel claim string {}", claim_string);
                let claim: PersistentVolumeClaim = serde_json::from_str(&claim_string)?;
                let claim_deployment = claim_api.create(&PostParams::default(), &claim).await?;

                let channel_statefulset_string = self
                    .reg
                    .render("channel_statefulset", &data_unit_render_args)?;
                debug!(
                    "rendered channel statefulset string {}",
                    channel_statefulset_string
                );
                let channel_statefulset: StatefulSet =
                    serde_json::from_str(channel_statefulset_string.as_str())?;
                let channel_statefulset = statefulset_api
                    .create(&PostParams::default(), &channel_statefulset)
                    .await?;

                let channel_service_string = self.reg.render("channel_service", node)?;
                debug!("rendered channel service string {}", channel_service_string);

                let channel_service: Service =
                    serde_json::from_str(channel_service_string.as_str())?;
                let channel_service = service_api
                    .create(&PostParams::default(), &channel_service)
                    .await?;
                Some(KubeChannelHander {
                    claim: claim_deployment,
                    statefulset: channel_statefulset,
                    service: channel_service,
                    db_repo: repo.clone(),
                })
            } else {
                None
            };


            // apply nodes
            let claim_string = self.reg.render(
                "claim",
                &ClaimRenderParams {
                    name: node.name.clone() + "-node-claim",
                },
            )?;
            debug!("rendered clam string {}", claim_string);
            let claim: PersistentVolumeClaim = serde_json::from_str(&claim_string)?;
            let claim_deployment = claim_api.create(&PostParams::default(), &claim).await?;

      
            let statefulset_string = self.reg.render("statefulset", &data_unit_render_args)?;
            debug!("rendered unit string {}", statefulset_string);

            let unit_statefulset: StatefulSet = serde_json::from_str(&statefulset_string)?;
            let unit_statefulset = statefulset_api
                .create(&PostParams::default(), &unit_statefulset)
                .await?;

            let node_record = Node{
                node_name: node.name.clone(),
                state: TrackerState::Init,
                node_type: NodeType::CoputeUnit,
                upstreams: upstreams.iter().map(|v|v.to_string()).collect(),
                created_at: cur_tm,
                updated_at: cur_tm,
            };

            repo.insert_node(node_record).await?;

            let service_string = self.reg.render("service", node)?;
            debug!("rendered unit service config {}", service_string);

            let unit_service: Service = serde_json::from_str(service_string.as_str())?;
            let unit_service = service_api
                .create(&PostParams::default(), &unit_service)
                .await?;

          

            let handler = KubeHandler {
                claim: claim_deployment,
                statefulset: unit_statefulset,
                service: unit_service,
                channel: channel_handler,
                db_repo: repo.clone(),
            };

            pipeline_ctl.handlers.insert(node.name.clone(), handler);
        }
        Ok(pipeline_ctl)
    }

    #[allow(refining_impl_trait)]
    async fn attach(&self, _namespace: &str, _graph: &Dag) -> Result<KubePipelineController<R>> {
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
    use crate::dbrepo::mongo::{MongoConfig, MongoRepo};
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

        let mongo_cfg = MongoConfig {
            mongo_url: "mongodb://localhost:27017".to_string(),
        };
        let client = Client::try_default().await.unwrap();
        let kube_driver= KubeDriver::<MongoRepo, MongoConfig>::new(client, mongo_cfg).await.unwrap();

        kube_driver.deploy("ntest", &dag).await.unwrap();
        //    kube_driver.clean("ntest").await.unwrap();
    }
}
