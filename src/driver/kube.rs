use super::{ChannelHandler, Driver, PipelineController, UnitHandler};
use crate::core::models::{DBConfig, Graph, GraphRepo, Node, NodeRepo, NodeType, TrackerState};
use crate::core::{models::DbRepo, ComputeUnit};
use crate::dag::Dag;
use crate::dbrepo::mongo::MongoRepo;
use crate::utils::IntoAnyhowResult;
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use handlebars::{Context, Handlebars, Helper, Output, RenderContext, RenderError};
use k8s_openapi::api::apps::v1::StatefulSet;
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
            db_repo: db_repo,
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

pub struct KubePipelineController<R>
where
    R: DbRepo,
{
    pub db_repo: R,
    handlers: HashMap<String, KubeHandler<R>>,
}

impl<R> KubePipelineController<R>
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

impl<R> PipelineController for KubePipelineController<R>
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

fn join_array(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // get parameter from helper or throw an error
    let param = h.param(0);

    match param {
        None => Ok(()),
        Some(args) => {
            let args = args.value().as_array().unwrap();
            let args_str: Vec<String> = args
                .iter()
                .map(|v| "\"".to_owned() + v.as_str().unwrap() + "\"")
                .collect();
            let rendered = format!("{}", args_str.join(","));
            out.write(rendered.as_ref())?;
            Ok(())
        }
    }
}

pub struct KubeDriver<'reg, R, DBC>
where
    R: DbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    reg: Handlebars<'reg>,
    client: Client,
    db_config: DBC,
    _phantom_data: PhantomData<R>,
}

impl<'reg, R, DBC> KubeDriver<'reg, R, DBC>
where
    R: DbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig,
{
    pub async fn new(client: Client, db_config: DBC) -> Result<KubeDriver<'reg, R, DBC>> {
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
        reg.register_helper("join_array", Box::new(join_array));
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
    DBC: Sized + Serialize + Send + Sync + DBConfig + 'static,
{
    node: &'a ComputeUnit,
    log_level: &'a str,
    db: DBC,
    run_id: &'a str,
    commnad: &'a str,
    args: Vec<&'a str>,
}

impl<R, DBC> Driver for KubeDriver<'_, R, DBC>
where
    R: DbRepo,
    DBC: Clone + Serialize + Send + Sync + DBConfig + 'static,
{
    #[allow(refining_impl_trait)]
    async fn deploy(&self, run_id: &str, graph: &Dag) -> Result<KubePipelineController<MongoRepo>> {
        Self::ensure_namespace_exit_and_clean(&self.client, run_id).await?;

        let repo = MongoRepo::new(self.db_config.clone(), run_id).await?;
        let statefulset_api: Api<StatefulSet> = Api::namespaced(self.client.clone(), run_id);
        let claim_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), run_id);
        let service_api: Api<Service> = Api::namespaced(self.client.clone(), run_id);

        // insert global record
        let cur_tm = Utc::now().timestamp();
        let graph_record = Graph {
            graph_json: graph.raw.clone(),
            created_at: cur_tm,
            updated_at: cur_tm,
        };
        repo.insert_global_state(graph_record).await?;

        let mut pipeline_ctl = KubePipelineController::new(repo.clone());
        for node in graph.iter() {
            if node.spec.cmd.len() == 0 {
                return Err(anyhow!("{} dont have command", &node.name));
            }
            let command = &node.spec.cmd[0];
            let args: Vec<&str> = node
                .spec
                .cmd
                .iter()
                .skip(1)
                .map(|arg| arg.as_str())
                .collect();
            let data_unit_render_args = NodeRenderParams {
                node,
                db: self.db_config.clone(),
                log_level: "debug",
                run_id,
                commnad: command,
                args: args,
            };
            let upstreams = graph.get_incomming_nodes(&node.name);
            let downstreams = graph.get_outgoing_nodes(&node.name);
            // apply channel
            let channel_handler = if upstreams.len() > 0 {
                //if node have no upstream node, no need to create channel points
                //channel node receive upstreams from upstream nodes
                let upstreams = upstreams
                    .iter()
                    .map(|node_name| {
                        graph
                            .get_node(node_name)
                            .expect("node added already before")
                    })
                    .map(|node| {
                        //<pod-name>.<service-name>.<namespace>.svc.cluster.local
                        (0..node.spec.replicas).map(|seq| {
                            format!(
                                "http://{}-statefulset-{}.{}-headless-service.{}.svc.cluster.local:80",
                                node.name, seq, node.name, run_id
                            )
                        })
                    })
                    .flatten()
                    .collect::<Vec<_>>();

                let node_stream = (0..node.spec.replicas)
                    .map(|seq| {
                        format!(
                            "http://{}-statefulset-{}.{}-headless-service.{}.svc.cluster.local:80",
                            node.name, seq, node.name, run_id
                        )
                    })
                    .collect::<Vec<_>>();
                let channel_record = Node {
                    node_name: node.name.clone() + "-channel",
                    state: TrackerState::Init,
                    node_type: NodeType::Channel,
                    upstreams: upstreams,
                    downstreams: node_stream, //only node read this channel
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

                let channel_service_string =
                    self.reg.render("channel_service", &data_unit_render_args)?;
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

            // compute unit only receive data from channel
            let channel_stream = (upstreams.len()>0).then(||{
                vec![format!(
                    "http://{}-channel-statefulset-{}.{}-channel-headless-service.{}.svc.cluster.local:80",
                      node.name, 0, node.name, run_id
                  )]
            }).unwrap_or_else(||vec![]);
            let outgoing_node_streams = downstreams
                    .iter()
                    .map(|node_name| {
                        graph
                            .get_node(node_name)
                            .expect("node added already before")
                    })
                    .map(|node| {
                        //<pod-name>.<service-name>.<namespace>.svc.cluster.local
                        format!(
                            "http://{}-channel-statefulset-{}.{}-channel-headless-service.{}.svc.cluster.local:80",
                            node.name, 0, node.name, run_id
                        )
                    })
                    .collect::<Vec<_>>();
            debug!("{}'s upstreams {:?}", node.name, upstreams);
            let node_record = Node {
                node_name: node.name.clone(),
                state: TrackerState::Init,
                node_type: NodeType::CoputeUnit,
                upstreams: channel_stream,
                downstreams: outgoing_node_streams,
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
    use mongodb::Client as MongoClient;
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
              "dependency": [],
              "spec": {
                "image": "jz-action/dummy_in:latest",
                "cmd": ["/dummy_in", "--log-level=debug"],
                "replicas":1
              }
            },
            {
              "name": "computeunit2",
              "node_type": "ComputeUnit",
              "dependency": [
                "computeunit1"
              ],
              "spec": {
                "image": "jz-action/dummy_out:latest",
                "cmd": ["/dummy_out", "--log-level=debug"],
                "replicas":1
              }
            }
          ]
        }
                        "#;
        let dag = Dag::from_json(json_str).unwrap();

        let mongo_cfg = MongoConfig {
            mongo_url: "mongodb://192.168.3.163:27017".to_string(),
        };

        let client = MongoClient::with_uri_str(mongo_cfg.connection_string())
            .await
            .unwrap();
        client.database("ntest").drop().await.unwrap();

        let client = Client::try_default().await.unwrap();
        let kube_driver = KubeDriver::<MongoRepo, MongoConfig>::new(client, mongo_cfg)
            .await
            .unwrap();

        kube_driver.deploy("ntest", &dag).await.unwrap();
        //    kube_driver.clean("ntest").await.unwrap();
    }
}
