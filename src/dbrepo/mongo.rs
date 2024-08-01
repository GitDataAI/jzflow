use crate::{
    core::models::{DBConfig, Graph, GraphRepo, Node, NodeRepo, DataRecord,DataState, DataRepo},
    utils::StdIntoAnyhowResult,
};
use anyhow::{anyhow, Result};
use mongodb::{bson::doc, error::ErrorKind, options::IndexOptions, Client, Collection, IndexModel};
use serde::Serialize;
use std::{ops::Deref, sync::Arc};
use serde_variant::to_variant_name;

const GRAPH_COL_NAME: &'static str = "graph";
const NODE_COL_NAME: &'static str = "node";
const DATA_COL_NAME: &'static str = "data";

#[derive(Clone)]
pub struct MongoRepo {
    graph_col: Collection<Graph>,
    node_col: Collection<Node>,
    data_col: Collection<DataRecord>,
}


#[derive(Clone, Serialize)]
pub struct MongoConfig {
    pub mongo_url: String,
}

impl MongoConfig {
    pub fn new(mongo_url: String) -> Self {
        MongoConfig { mongo_url }
    }
}

impl DBConfig for MongoConfig {
    fn connection_string(&self) -> &str {
        return &self.mongo_url;
    }
}

impl MongoRepo {
    pub async fn new<DBC>(config: DBC, db_name: &str) -> Result<Self>
    where
        DBC: DBConfig,
    {
        let client = Client::with_uri_str(config.connection_string()).await?;
        let database = client.database(db_name);
        let graph_col: Collection<Graph> = database.collection(&GRAPH_COL_NAME);
        let node_col: Collection<Node> = database.collection(&NODE_COL_NAME);
        let data_col: Collection<DataRecord> = database.collection(&DATA_COL_NAME);

        //create index
        let idx_opts = IndexOptions::builder()
            .unique(true)
            .name("idx_node_name_unique".to_owned())
            .build();

        let index = IndexModel::builder()
            .keys(doc! { "node_name": 1 })
            .options(idx_opts)
            .build();

        if let Err(e) = node_col.create_index(index).await {
            match *e.kind {
                ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                e => {
                    return Err(anyhow!("create index error {}", e));
                }
            }
        }

        Ok(MongoRepo {
            graph_col,
            node_col,
            data_col,
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
        match self.node_col.find_one(doc! {"node_name":name}).await {
            Ok(None) => Err(anyhow!("node not exit")),
            Ok(Some(val)) => Ok(val),
            Err(e) => Err(e.into()),
        }
    }
}

impl DataRepo for MongoRepo {
    async fn get_avaiable_data(&self, node_name: &str)-> Result<Option<String>> {
        let update = doc! {
            "$set": { "state": "Assigned" },
        };

        let result =  self.data_col.find_one_and_update( doc! {"node_name":node_name,"state": "Received"},  update ).await?;
        Ok(result.map(|record|record.id))
    }

    async fn insert_new_path(&self, record: &DataRecord)-> Result<()> {
        self.data_col.insert_one(record).await.map(|_|()).anyhow()
    }

    async fn update_state(&self, node_name: &str, id: &str, state: DataState) ->Result<()> {
        let update = doc! {
            "$set": { "state":  to_variant_name(&state)?  },
        };

        self.data_col.find_one_and_update( doc! {"node_name":node_name,"id": id},  update ).await.map(|_|()).anyhow()
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

impl<T: DataRepo> DataRepo for Arc<T> {
    async fn get_avaiable_data(&self, node_name: &str)-> Result<Option<String>>{
        self.deref().get_avaiable_data(node_name).await
    }

    async fn insert_new_path(&self, record: &DataRecord)-> Result<()>{
        self.deref().insert_new_path(record).await
    }

    async fn update_state(&self, node_name: &str, id: &str, state: DataState) ->Result<()>{
        self.deref().update_state(node_name, id,  state).await
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tracing_subscriber;

    fn is_send<T>()where T:Send+Sync{}
    #[tokio::test]
    async fn test_render() {
        is_send::<MongoRepo>();
    }
}
