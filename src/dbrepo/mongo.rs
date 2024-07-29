use crate::{
    core::models::{DBConfig, Graph, GraphRepo, Node, NodeRepo},
    utils::StdIntoAnyhowResult,
};
use anyhow::{anyhow, Result};
use mongodb::{bson::doc, options::IndexOptions, Client, Collection, IndexModel};
use serde::Serialize;
use std::{ ops::Deref, sync::Arc};
use tracing::error;

const GRAPH_COL_NAME: &'static str = "graph";
const NODE_COL_NAME: &'static str = "node";

#[derive(Clone)]
pub struct MongoRepo {
    graph_col: Collection<Graph>,
    node_col: Collection<Node>,
}

#[derive(Clone, Serialize)]
pub struct MongoConfig {
    pub mongo_url: String,
}

impl MongoConfig {
    pub fn new(mongo_url: String) -> Self {
        MongoConfig{
            mongo_url,
        }
    }
}
impl DBConfig for MongoConfig {
    fn connection_string(&self) -> &str {
        return &self.mongo_url;
    }
}

impl MongoRepo {
    pub async fn new<DBC>(config: DBC, db_name: &str) -> Result<Self>
    where DBC: DBConfig
     {
        let client = Client::with_uri_str(config.connection_string()).await?;
        let database = client.database(db_name);
        let graph_col: Collection<Graph> = database.collection(&GRAPH_COL_NAME);
        let node_col: Collection<Node> = database.collection(&NODE_COL_NAME);

        //create index
        let idx_opts = IndexOptions::builder().unique(true).build();

        let index = IndexModel::builder()
            .keys(doc! { "name": 1 })
            .options(idx_opts)
            .build();

        if let Err(e) = node_col.create_index(index).await {
            error!("write index {e}");
        }

        Ok(MongoRepo {
            graph_col,
            node_col,
        })
    }
}

impl GraphRepo for MongoRepo
{
    async fn insert_global_state(&self, state: Graph) -> Result<()> {
        self.graph_col.insert_one(state).await.map(|_| ()).anyhow()
    }

    async fn get_global_state(&self) -> Result<Graph> {
        match self.graph_col.find_one(doc! {}).await {
            Ok(None) => Err(anyhow!("global state not exit")),
            Ok(Some(val)) => Ok(val),
            Err(e) => Err(e.into()),
        }
    }
}

impl NodeRepo for MongoRepo
{
    async fn insert_node(&self, state: Node) -> Result<()> {
        self.node_col.insert_one(state).await.map(|_| ()).anyhow()
    }

    async fn get_node_by_name(&self, name: &str) -> Result<Node> {
        match self.node_col.find_one(doc! {name:name}).await {
            Ok(None) => Err(anyhow!("node not exit")),
            Ok(Some(val)) => Ok(val),
            Err(e) => Err(e.into()),
        }
    }
}

impl<T: NodeRepo> NodeRepo for Arc<T> {
    async fn insert_node(&self, state: Node) -> Result<()> {
        self.deref().insert_node(state).await
    }

    async fn get_node_by_name(&self, name: &str) -> Result<Node> {
        self.deref().get_node_by_name(name).await
    }
}

impl<T: GraphRepo> GraphRepo for Arc<T> {
    async fn insert_global_state(&self, state: Graph) -> Result<()> {
        self.deref().insert_global_state(state).await
    }

    async fn get_global_state(&self) -> Result<Graph> {
        self.deref().get_global_state().await
    }
}
