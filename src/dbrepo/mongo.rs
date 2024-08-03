use crate::{
    core::models::{
        DBConfig,
        DataRecord,
        DataRepo,
        DataState,
        Direction,
        Graph,
        GraphRepo,
        Node,
        NodeRepo,
    },
    utils::StdIntoAnyhowResult,
};
use anyhow::{
    anyhow,
    Result,
};
use futures::TryStreamExt;
use mongodb::{
    bson::doc,
    error::ErrorKind,
    options::IndexOptions,
    Client,
    Collection,
    IndexModel,
};
use serde::Serialize;
use serde_variant::to_variant_name;
use tokio_stream::StreamExt;

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

        {
            //create index for nodes
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
        }

        {
            //create  index  for data
            let idx_opts = IndexOptions::builder()
                .name("idx_node_name_state_direction".to_owned())
                .build();

            let index = IndexModel::builder()
                .keys(doc! { "node_name": 1,"state": 1,"direction": 1})
                .options(idx_opts)
                .build();

            if let Err(e) = data_col.create_index(index).await {
                match *e.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                    e => {
                        return Err(anyhow!("create index error {}", e));
                    }
                }
            }
        }

        {
            //create  index  for data
            let idx_opts = IndexOptions::builder()
                .name("idx_node_name_id_direction".to_owned())
                .build();

            let index = IndexModel::builder()
                .keys(doc! { "node_name": 1,"id": 1,"direction": 1})
                .options(idx_opts)
                .build();

            if let Err(e) = data_col.create_index(index).await {
                match *e.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                    e => {
                        return Err(anyhow!("create index error {}", e));
                    }
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
    async fn find_data_and_mark_state(
        &self,
        node_name: &str,
        direction: Direction,
        state: DataState,
    ) -> Result<Option<DataRecord>> {
        let update = doc! {
            "$set": { "state": to_variant_name(&state)? },
        };

        self
            .data_col
            .find_one_and_update(
                doc! {"node_name":node_name, "state": doc! {"$in": ["Received","PartialSent"]}, "direction":to_variant_name(&direction)?},
                update
            )
            .await.anyhow()
    }

    async fn find_by_node_id(
        &self,
        node_name: &str,
        id: &str,
        direction: Direction,
    ) -> Result<Option<DataRecord>> {
        self.data_col
            .find_one(
                doc! {"id": id,"node_name":node_name, "direction": to_variant_name(&direction)?},
            )
            .await
            .anyhow()
    }

    async fn list_by_node_name_and_state(
        &self,
        node_name: &str,
        state: DataState,
    ) -> Result<Vec<DataRecord>> {
        let cursor = self
            .data_col
            .find(doc! {"node_name":node_name, "state": to_variant_name(&state)?})
            .await?;
        cursor.try_collect().await.anyhow()
    }

    async fn count_pending(&self, node_name: &str, direction: Direction) -> Result<usize> {
        self.data_col.count_documents(doc! {"node_name":node_name,"state": doc! {"$in": ["Received","PartialSent"]}, "direction": to_variant_name(&direction)?}).await.map(|count|count as usize).anyhow()
    }

    async fn update_state(
        &self,
        node_name: &str,
        id: &str,
        direction: Direction,
        state: DataState,
        sent: Option<Vec<&str>>,
    ) -> Result<()> {
        let mut update_fields = doc! {"state":  to_variant_name(&state)?};
        if let Some(sent) = sent {
            update_fields.insert("sent", sent);
        }

        self.data_col
            .find_one_and_update(
                doc! {"node_name":node_name,"id": id, "direction": to_variant_name(&direction)?},
                doc! {"$set": update_fields},
            )
            .await
            .map(|_| ())
            .anyhow()
    }

    async fn insert_new_path(&self, record: &DataRecord) -> Result<()> {
        self.data_col.insert_one(record).await.map(|_| ()).anyhow()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send<T>()
    where
        T: Send + Sync,
    {
    }
    #[tokio::test]
    async fn test_render() {
        is_send::<MongoRepo>();
    }
}
