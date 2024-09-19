use crate::{
    core::{
        db::{
            Graph,
            Node,
            NodeType,
            Repo,
            TrackerState,
        },
    },
    dag::Dag,
    dbrepo::MongoRunDbRepo,
    driver::{
        kube_option::KubeOptions,
        kube_pipe::KubePipelineController,
        kube_util::{
            join_array,
            merge_storage_options,
            ClaimRenderParams,
            NodeRenderParams,
        },
        kube::KubeHandler,
    },
};
use anyhow::anyhow;
use chrono::Utc;
use handlebars::Handlebars;
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{
        Namespace,
        PersistentVolumeClaim,
        Service,
    },
};
use kube::{
    api::{
        DeleteParams,
        PostParams,
    },
    runtime::reflector::Lookup,
    Api,
    Client,
};
use std::marker::PhantomData;
use tokio_retry::{
    strategy::ExponentialBackoff,
    Retry,
};
use tracing::debug;
use crate::driver::Driver;

pub struct KubeDriver<R>
where
    R: Repo,
{
    reg: Handlebars<'static>,
    client: Client,

    options: KubeOptions,
    _phantom_data: PhantomData<R>,
}

impl<R> KubeDriver<R>
where
    R: Repo,
{
    pub async fn new(client: Client, options: KubeOptions) -> anyhow::Result<KubeDriver<R>> {
        let mut reg = Handlebars::new();
        reg.register_template_string("claim", include_str!("kubetpl/claim.tpl"))?;

        reg.register_template_string("statefulset", include_str!("kubetpl/statefulset.tpl"))?;
        reg.register_template_string("service", include_str!("kubetpl/service.tpl"))?;
        reg.register_helper("join_array", Box::new(join_array));
        Ok(KubeDriver {
            reg,
            client,
            options,
            _phantom_data: PhantomData,
        })
    }

    async fn ensure_namespace_exit_and_clean(client: &Client, ns: &str) -> anyhow::Result<()> {
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
            Retry::spawn(retry_strategy, || async {
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

impl<R> Clone for KubeDriver<R>
where
    R: Repo,
{
    fn clone(&self) -> Self {
        Self{
            reg: self.reg.clone(),
            client: self.client.clone(),
            options: self.options.clone(),
            _phantom_data: self._phantom_data.clone(),
        }
    }
}

impl<R> Driver for KubeDriver<R>
where
    R: Repo,
{
    #[allow(refining_impl_trait)]
    async fn deploy(
        &self,
        run_id: &str,
        graph: &Dag,
    ) -> anyhow::Result<KubePipelineController<MongoRunDbRepo>> {
        Self::ensure_namespace_exit_and_clean(&self.client, run_id).await?;

        let db_url = self.options.db_url.clone() + "/" + run_id;
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
        let topo_sort_nodes = graph.topo_sort_nodes();
        let mut pipeline_ctl =
            KubePipelineController::new(repo.clone(), self.client.clone(), topo_sort_nodes);
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

            // apply nodes
            let claim_string = self.reg.render(
                "claim",
                &ClaimRenderParams {
                    storage: merge_storage_options(&self.options.storage, &node.spec.storage),
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
            let outgoing_node_streams = down_nodes
                .iter()
                .map(|node_name| {
                    format!(
                        "http://{}-service.{}.svc.cluster.local:80",
                        node_name, run_id
                    )
                })
                .collect::<Vec<_>>();

            let node_record = Node {
                node_name: node.name.clone(),
                state: TrackerState::Init,
                node_type: NodeType::CoputeUnit,
                up_nodes: up_nodes.iter().map(|v| v.to_string()).collect(),
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
    ) -> anyhow::Result<KubePipelineController<MongoRunDbRepo>> {
        let db_url = self.options.db_url.clone() + "/" + run_id;
        let repo = MongoRunDbRepo::new(db_url.as_str())
            .await
            .map_err(|err| anyhow!("create database fail {err}"))?;
        let statefulset_api: Api<StatefulSet> = Api::namespaced(self.client.clone(), run_id);
        let claim_api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), run_id);
        let service_api: Api<Service> = Api::namespaced(self.client.clone(), run_id);

        let topo_sort_nodes = graph.topo_sort_nodes();
        let mut pipeline_ctl =
            KubePipelineController::new(repo.clone(), self.client.clone(), topo_sort_nodes);
        for node in graph.iter() {
            // apply nodes
            let claim_deployment = claim_api
                .get((node.name.clone() + "-node-claim").as_str())
                .await?;
            let unit_statefulset = statefulset_api
                .get((node.name.clone() + "-statefulset").as_str())
                .await?;

            let unit_service = service_api
                .get((node.name.clone() + "-service").as_str())
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
                db_repo: repo.clone(),
            };

            pipeline_ctl.handlers.insert(node.name.clone(), handler);
        }
        Ok(pipeline_ctl)
    }

    async fn clean(&self, ns: &str) -> anyhow::Result<()> {
        let client: Client = Client::try_default().await?;
        let namespaces: Api<Namespace> = Api::all(client.clone());
        if namespaces.get(ns).await.is_ok() {
            namespaces
                .delete(ns, &DeleteParams::default())
                .await
                .map(|_| ())
                .map_err(|e| anyhow!("{}", e.to_string()))?;
        }
        Ok(())
    }
}
