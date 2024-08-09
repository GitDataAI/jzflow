use super::{
    ChannelHandler,
    Driver,
    PipelineController,
    UnitHandler,
    NodeStatus,
    PodStauts,
};
use crate::{
    core::{
        db::{
            Graph,
            GraphRepo,
            JobDbRepo,
            Node,
            NodeType,
            TrackerState,
        },
        ComputeUnit,
    },
    dag::Dag,
    dbrepo::MongoRunDbRepo,
    utils::{IntoAnyhowResult, StdIntoAnyhowResult},
};
use anyhow::{
    anyhow,
    Result,
};
use chrono::prelude::*;
use handlebars::{
    Context,
    Handlebars,
    Helper,
    Output,
    RenderContext,
    RenderError,
};
use k8s_openapi::api::{
    apps::v1::StatefulSet, core::v1::{
        Namespace, PersistentVolumeClaim, Pod, Service
    }
};
use kube::{
    api::{
        DeleteParams, ListParams, PostParams
    },
    runtime::reflector::Lookup,
    Api,
    Client,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    default::Default,
    marker::PhantomData,
    sync::{
        Arc,
        Mutex,
    },
};
use tokio_retry::{
    strategy::ExponentialBackoff,
    Retry,
};
use tracing::debug;

pub struct KubeChannelHander<R>
where
    R: JobDbRepo,
{
    pub(crate) client: Client,
    pub(crate) node_name:String,
    pub(crate) namespace: String,
    pub(crate) stateset_name: String,
    pub(crate) claim_name: String,
    pub(crate) service_name: String,
    pub(crate) db_repo: R,
}

impl<R> ChannelHandler for KubeChannelHander<R>
where
    R: JobDbRepo,
{
    fn name(&self) -> &str {
        &self.node_name
    }
    
    async fn status(&self) -> Result<NodeStatus> {
        let statefulset_api: Api<StatefulSet> = Api::namespaced(self.client.clone(), &self.namespace);
        let claim_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), &self.namespace);
        let service_api: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        let pods_api:Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let statefulset = statefulset_api.get(&self.stateset_name).await.anyhow()?;
        let selector = statefulset.spec.as_ref().unwrap().selector.match_labels.as_ref().expect("set in template")
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>()
        .join(",");

        let list_params = ListParams::default().labels(&selector);
        let pods = pods_api.list(&list_params).await.anyhow()?;

        let pvc = claim_api.get(&self.claim_name).await.anyhow()?;
        let cap = pvc.status.unwrap().capacity.unwrap().get("storage").map(|cap|cap.0.clone()).unwrap_or_default();

        let mut node_status = NodeStatus {
            name: self.node_name.clone(),
            replicas: statefulset.spec.as_ref().and_then(|spec|spec.replicas).unwrap_or_default() as u32,
            storage: cap,
            pods: HashMap::new(),
        };


        for pod in pods {
            let pod_name = pod.metadata.name.expect("set in template");
            let phase = pod.status.and_then(|status|status.phase).unwrap_or_default();

            let pod_status = PodStauts{
                state: phase,
                disk_usage:0,
                cpu_usage:0,
                memory_usage:0
            };
            node_status.pods.insert(pod_name, pod_status);
        }
        Ok(node_status)
    }
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
    R: JobDbRepo,
{
    pub(crate) client: Client,
    pub(crate) node_name:String,
    pub(crate) namespace: String,
    pub(crate) stateset_name: String,
    pub(crate) claim_name: String,
    pub(crate) service_name: String,
    pub(crate) db_repo: R,
    pub(crate) channel: Option<KubeChannelHander<R>>,
}

impl<R> UnitHandler for KubeHandler<R>
where
    R: JobDbRepo,
{
    fn name(&self) -> &str {
        &self.node_name
    }

    async fn status(&self) -> Result<NodeStatus> {
        let statefulset_api: Api<StatefulSet> = Api::namespaced(self.client.clone(), &self.namespace);
        let claim_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), &self.namespace);
        let service_api: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        let pods_api:Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let statefulset = statefulset_api.get(&self.stateset_name).await.anyhow()?;
        let selector = statefulset.spec.as_ref().unwrap().selector.match_labels.as_ref().expect("set in template")
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>()
        .join(",");

        let list_params = ListParams::default().labels(&selector);
        let pods = pods_api.list(&list_params).await.anyhow()?;

        let pvc = claim_api.get(&self.claim_name).await.anyhow()?;
        let cap = pvc.status.unwrap().capacity.unwrap().get("storage").map(|cap|cap.0.clone()).unwrap_or_default();

        let mut node_status = NodeStatus {
            name: self.node_name.clone(),
            replicas: statefulset.spec.as_ref().and_then(|spec|spec.replicas).unwrap_or_default() as u32,
            storage: cap,
            pods: HashMap::new(),
        };

        for pod in pods {
            let pod_name = pod.metadata.name.expect("set in template");
            let phase = pod.status.and_then(|status|status.phase).unwrap_or_default();

            let pod_status = PodStauts{
                state: phase,
                disk_usage:0,
                cpu_usage:0,
                memory_usage:0
            };
            node_status.pods.insert(pod_name, pod_status);
        }
        Ok(node_status)
    }

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
    R: JobDbRepo,
{
    client: Client,
    db_repo: R,
    handlers: HashMap<String, KubeHandler<R>>,
}

impl<R> KubePipelineController<R>
where
    R: JobDbRepo,
{
    fn new(repo: R, client: Client) -> Self {
        Self {
            db_repo: repo,
            client,
            handlers: Default::default(),
        }
    }
}

impl<R> PipelineController for KubePipelineController<R>
where
    R: JobDbRepo,
{
    type Output = KubeHandler<R>;

    //todo use iter
    fn nodes(&self) -> Result<Vec<String>> {
        anyhow::Ok(self.handlers.keys().map(|a| a.to_string()).collect())
    }

    async fn get_node(&self, id: &str) -> Result<&KubeHandler<R>> {
        self.handlers.get(id).anyhow("id not found")
    }

    async fn get_node_mut(&mut self, id: &str) -> Result<&mut KubeHandler<R>> {
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

#[derive(Clone)]
pub struct KubeDriver<R>
where
    R: JobDbRepo,
{
    reg: Handlebars<'static>,
    client: Client,
    db_url: String,
    _phantom_data: PhantomData<R>,
}

impl<R> KubeDriver<R>
where
    R: JobDbRepo,
{
    pub async fn new(client: Client, db_url: &str) -> Result<KubeDriver<R>> {
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
            db_url: db_url.to_string(),
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
                    Err(err) => {
                        if err.to_string().contains("not found") {
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
struct NodeRenderParams<'a> {
    node: &'a ComputeUnit,
    log_level: &'a str,
    db_url: &'a str,
    run_id: &'a str,
}

impl<R> Driver for KubeDriver<R>
where
    R: JobDbRepo,
{
    #[allow(refining_impl_trait)]
    async fn deploy(
        &self,
        run_id: &str,
        graph: &Dag,
    ) -> Result<KubePipelineController<MongoRunDbRepo>> {
        Self::ensure_namespace_exit_and_clean(&self.client, run_id).await?;

        let db_url = self.db_url.to_string() + "/" + run_id;
        let repo = MongoRunDbRepo::new(db_url.as_str())
            .await
            .map_err(|err| anyhow!("create database fail {err}"))?;
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
        repo.insert_global_state(&graph_record).await?;

        let mut pipeline_ctl = KubePipelineController::new(repo.clone(), self.client.clone());
        for node in graph.iter() {
            if node.spec.command.is_empty() {
                return Err(anyhow!("{} dont have command", &node.name));
            }

            let data_unit_render_args = NodeRenderParams {
                node,
                db_url: db_url.as_str(),
                log_level: "debug",
                run_id,
            };
            let up_nodes = graph.get_incomming_nodes(&node.name);
            let down_nodes = graph.get_outgoing_nodes(&node.name);
            // apply channel
            let (channel_handler, channel_nodes) = if up_nodes.len() > 0 {
                //if node have no upstream node, no need to create channel points
                //channel node receive upstreams from upstream nodes
                let upstreams = up_nodes
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
                let channel_node_name = node.name.clone() + "-channel";
                let channel_record = Node {
                    node_name: channel_node_name.clone(),
                    state: TrackerState::Init,
                    node_type: NodeType::Channel,
                    up_nodes: up_nodes.iter().map(|node| node.to_string()).collect(),
                    incoming_streams: upstreams,
                    outgoing_streams: node_stream, //only node read this channel
                    created_at: cur_tm,
                    updated_at: cur_tm,
                };
                repo.insert_node(&channel_record).await?;

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
                (
                    Some(KubeChannelHander {
                        node_name: channel_node_name.clone(),
                        client: self.client.clone(),
                        namespace: run_id.to_string().clone(),
                        stateset_name: channel_statefulset
                            .name()
                            .expect("set name in template")
                            .to_string(),
                        claim_name: claim_deployment
                            .name()
                            .expect("set name in template")
                            .to_string(),
                        service_name: channel_service
                            .name()
                            .expect("set name in template")
                            .to_string(),
                        db_repo: repo.clone(),
                    }),
                    vec![channel_node_name],
                )
            } else {
                (None, vec![])
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
            let channel_stream = (up_nodes.len()>0).then(||{
                vec![format!(
                    "http://{}-channel-statefulset-{}.{}-channel-headless-service.{}.svc.cluster.local:80",
                      node.name, 0, node.name, run_id
                  )]
            }).unwrap_or_else(||vec![]);
            let outgoing_node_streams = down_nodes
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

            let node_record = Node {
                node_name: node.name.clone(),
                state: TrackerState::Init,
                node_type: NodeType::CoputeUnit,
                up_nodes: channel_nodes,
                incoming_streams: channel_stream,
                outgoing_streams: outgoing_node_streams,
                created_at: cur_tm,
                updated_at: cur_tm,
            };

            repo.insert_node(&node_record).await?;

            let service_string = self.reg.render("service", node)?;
            debug!("rendered unit service config {}", service_string);

            let unit_service: Service = serde_json::from_str(service_string.as_str())?;
            let unit_service = service_api
                .create(&PostParams::default(), &unit_service)
                .await?;

            let handler = KubeHandler {
                node_name: node.name.clone(),
                client: self.client.clone(),
                namespace: run_id.to_string().clone(),
                stateset_name: unit_statefulset
                    .name()
                    .expect("set name in template")
                    .to_string(),
                claim_name: claim_deployment
                    .name()
                    .expect("set name in template")
                    .to_string(),
                service_name: unit_service
                    .name()
                    .expect("set name in template")
                    .to_string(),
                channel: channel_handler,
                db_repo: repo.clone(),
            };

            pipeline_ctl.handlers.insert(node.name.clone(), handler);
        }
        Ok(pipeline_ctl)
    }

    #[allow(refining_impl_trait)]
    async fn attach(
        &self,
        run_id: &str,
        graph: &Dag,
    ) -> Result<KubePipelineController<MongoRunDbRepo>> {
        let db_url = self.db_url.to_string() + "/" + run_id;
        let repo = MongoRunDbRepo::new(db_url.as_str())
            .await
            .map_err(|err| anyhow!("create database fail {err}"))?;
        let statefulset_api: Api<StatefulSet> = Api::namespaced(self.client.clone(), run_id);
        let claim_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), run_id);
        let service_api: Api<Service> = Api::namespaced(self.client.clone(), run_id);

        let mut pipeline_ctl = KubePipelineController::new(repo.clone(), self.client.clone());
        for node in graph.iter() {
            let up_nodes = graph.get_incomming_nodes(&node.name);
            // query channel
            let channel_handler = if up_nodes.len() > 0 {
                let claim_deployment = claim_api
                    .get((node.name.clone() + "-channel-claim").as_str())
                    .await?;
                let channel_statefulset = statefulset_api
                    .get((node.name.clone() + "-channel-statefulset").as_str())
                    .await?;
                let channel_service = service_api
                    .get((node.name.clone() + "-channel-headless-service").as_str())
                    .await?;

                Some(KubeChannelHander {
                    node_name:node.name.clone() + "-channel",
                    client: self.client.clone(),
                    namespace: run_id.to_string().clone(),
                    stateset_name: channel_statefulset
                        .name()
                        .expect("set name in template")
                        .to_string(),
                    claim_name: claim_deployment
                        .name()
                        .expect("set name in template")
                        .to_string(),
                    service_name: channel_service
                        .name()
                        .expect("set name in template")
                        .to_string(),
                    db_repo: repo.clone(),
                })
            } else {
                None
            };

            // apply nodes
            let claim_deployment = claim_api
                .get((node.name.clone() + "-node-claim").as_str())
                .await?;
            let unit_statefulset = statefulset_api
                .get((node.name.clone() + "-statefulset").as_str())
                .await?;

            let unit_service = service_api
                .get((node.name.clone() + "-headless-service").as_str())
                .await?;

            let handler = KubeHandler {
                node_name: node.name.clone(),
                client: self.client.clone(),
                namespace: run_id.to_string().clone(),
                stateset_name: unit_statefulset
                    .name()
                    .expect("set name in template")
                    .to_string(),
                claim_name: claim_deployment
                    .name()
                    .expect("set name in template")
                    .to_string(),
                service_name: unit_service
                    .name()
                    .expect("set name in template")
                    .to_string(),
                channel: channel_handler,
                db_repo: repo.clone(),
            };

            pipeline_ctl.handlers.insert(node.name.clone(), handler);
        }
        Ok(pipeline_ctl)
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

    use crate::dbrepo::MongoRunDbRepo;

    use super::*;

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
              "name": "dummy-in",
              "spec": {
                "image": "gitdatateam/dummy_in:latest",
                "command":"/dummy_in",
                "args": ["--log-level=debug", "--total-count=100"]
              }
            },   {
              "name": "copy-in-place",
              "node_type": "ComputeUnit",
              "dependency": [
                "dummy-in"
              ],
              "spec": {
                "image": "gitdatateam/copy_in_place:latest",
                "command":"/copy_in_place",
                "replicas": 3,
                "args": ["--log-level=debug"]
              },
              "channel":{
                "cache_type":"Memory"
              }
            },
            {
              "name": "dummy-out",
              "node_type": "ComputeUnit",
              "dependency": [
                "copy-in-place"
              ],
              "spec": {
                "image": "gitdatateam/dummy_out:latest",
                "command":"/dummy_out",
                "replicas": 3,
                "args": ["--log-level=debug"]
              },
              "channel":{
                "cache_type":"Memory"
              }
            }
          ]
        }
                        "#;
        let dag = Dag::from_json(json_str).unwrap();

        let db_url = "mongodb://192.168.3.163:27017";
        let client = MongoClient::with_uri_str(db_url.to_string() + "/ntest")
            .await
            .unwrap();
        client.database("ntest").drop().await.unwrap();

        let client = Client::try_default().await.unwrap();
        let kube_driver = KubeDriver::<MongoRunDbRepo>::new(client, db_url)
            .await
            .unwrap();
        kube_driver.deploy("ntest", &dag).await.unwrap();
        //    kube_driver.clean("ntest").await.unwrap();
    }

    fn is_send_sync<T: Send + Sync>(v: T) {}
}
