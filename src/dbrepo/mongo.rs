use crate::{
    core::models::{Graph, GraphRepo, Node, NodeRepo},
    utils::StdIntoAnyhowResult,
};
use anyhow::{anyhow, Result};
use mongodb::{bson::doc, options::IndexOptions, Client, Collection, IndexModel};
use tracing::{error, info};

const GRAPH_COL_NAME: &'static str = "graph";
const NODE_COL_NAME: &'static str = "node";

pub struct MongoRepo {
    graph_col: Collection<Graph>,
    node_col: Collection<Node>,
}

impl MongoRepo {
    pub async fn new(mongo_url: &str, db_name: &str) -> Result<Self> {
        let client = Client::with_uri_str(mongo_url).await?;
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

impl GraphRepo for MongoRepo {
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

impl NodeRepo for MongoRepo {
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
