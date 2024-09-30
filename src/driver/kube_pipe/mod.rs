use crate::{
    core::db::Repo,
    driver::{
        kube::KubeHandler,
        PipelineController,
        UnitHandler,
    },
    utils::IntoAnyhowResult,
};
use futures::future;
use kube::Client;
use std::collections::HashMap;

pub struct KubePipelineController<R>
where
    R: Repo,
{
    _client: Client,
    _db_repo: R,
    topo_sort_nodes: Vec<String>,
    pub(crate) handlers: HashMap<String, KubeHandler<R>>,
}
impl<R> KubePipelineController<R>
where
    R: Repo,
{
    pub fn new(db_repo: R, client: Client, topo_sort_nodes: Vec<String>) -> Self {
        Self {
            _client: client,
            _db_repo: db_repo,
            topo_sort_nodes,
            handlers: HashMap::new(),
        }
    }
}

impl<R> PipelineController for KubePipelineController<R>
where
    R: Repo,
{
    type Output = KubeHandler<R>;

    async fn start(&self) -> anyhow::Result<()> {
        future::try_join_all(self.handlers.iter().map(|handler| handler.1.start()))
            .await
            .map(|_| ())
    }
    //todo use iter
    fn nodes_in_order(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.topo_sort_nodes.clone())
    }

    async fn get_node(&self, id: &str) -> anyhow::Result<&KubeHandler<R>> {
        self.handlers.get(id).anyhow("id not found")
    }

    async fn get_node_mut(&mut self, id: &str) -> anyhow::Result<&mut KubeHandler<R>> {
        self.handlers.get_mut(id).anyhow("id not found")
    }
}
