use crate::{
    core::db::{
        DataRecord,
        DataRepo,
        DataState,
        Direction,
        Graph,
        GraphRepo,
        Node,
        NodeRepo,
        TrackerState,
    },
    utils::StdIntoAnyhowResult,
};
use anyhow::{
    anyhow,
    Result,
};
use chrono::{
    Duration,
    Utc,
};
use futures::TryStreamExt;
use mongodb::{
    bson::doc,
    error::ErrorKind,
    options::{
        ClientOptions,
        IndexOptions,
    },
    Client,
    Collection,
    IndexModel,
};
use serde_variant::to_variant_name;

const GRAPH_COL_NAME: &str = "graph";
const NODE_COL_NAME: &str = "node";
const DATA_COL_NAME: &str = "data";

#[derive(Clone)]
pub struct MongoRunDbRepo {
    graph_col: Collection<Graph>,
    node_col: Collection<Node>,
    data_col: Collection<DataRecord>,
}

impl MongoRunDbRepo {
    pub async fn new(connectstring: &str) -> Result<Self> {
        let options = ClientOptions::parse(connectstring).await?;
        let database = options
            .default_database
            .as_ref()
            .expect("set db name in url")
            .clone();
        let client = Client::with_options(options)?;
        let database = client.database(database.as_str());
        let graph_col: Collection<Graph> = database.collection(GRAPH_COL_NAME);
        let node_col: Collection<Node> = database.collection(NODE_COL_NAME);
        let data_col: Collection<DataRecord> = database.collection(DATA_COL_NAME);

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

            if let Err(err) = node_col.create_index(index).await {
                match *err.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                    err => {
                        return Err(anyhow!("create index error {err}"));
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

            if let Err(err) = data_col.create_index(index).await {
                match *err.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                    err => {
                        return Err(anyhow!("create index error {err}"));
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

            if let Err(err) = data_col.create_index(index).await {
                match *err.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => {}
                    err => {
                        return Err(anyhow!("create index error {err}"));
                    }
                }
            }
        }

        Ok(MongoRunDbRepo {
            graph_col,
            node_col,
            data_col,
        })
    }
}

impl GraphRepo for MongoRunDbRepo {
    async fn insert_global_state(&self, graph: &Graph) -> Result<()> {
        self.graph_col.insert_one(graph).await.map(|_| ()).anyhow()
    }

    async fn get_global_state(&self) -> Result<Graph> {
        match self.graph_col.find_one(doc! {}).await {
            Ok(None) => Err(anyhow!("global state not exit")),
            Ok(Some(val)) => Ok(val),
            Err(err) => Err(err.into()),
        }
    }
}

impl NodeRepo for MongoRunDbRepo {
    async fn insert_node(&self, state: &Node) -> Result<()> {
        self.node_col.insert_one(state).await.map(|_| ()).anyhow()
    }

    async fn get_node_by_name(&self, name: &str) -> Result<Node> {
        match self.node_col.find_one(doc! {"node_name":name}).await {
            Ok(None) => Err(anyhow!("node not exit")),
            Ok(Some(val)) => Ok(val),
            Err(err) => Err(err.into()),
        }
    }

    async fn update_node_by_name(&self, name: &str, state: TrackerState) -> Result<()> {
        let update = doc! {
            "$set": {
                "state": to_variant_name(&state)?,
                "updated_at":Utc::now().timestamp(),
            },
        };

        self.node_col
            .update_one(doc! {"node_name":name}, update)
            .await
            .map(|_| ())
            .anyhow()
    }
}

impl DataRepo for MongoRunDbRepo {
    async fn find_data_and_mark_state(
        &self,
        node_name: &str,
        direction: &Direction,
        state: &DataState,
    ) -> Result<Option<DataRecord>> {
        let update = doc! {
            "$set": {
                "state": to_variant_name(&state)?,
                "updated_at":Utc::now().timestamp(),
            },
        };

        self
            .data_col
            .find_one_and_update(
                doc! {"node_name":node_name, "state": doc! {"$in": ["Received","PartialSent"]}, "direction":to_variant_name(&direction)?},
                update
            ).await
            .anyhow()
    }

    async fn revert_no_success_sent(&self, node_name: &str, direction: &Direction) -> Result<u64> {
        let update = doc! {
            "$set": {
                "state": to_variant_name(&DataState::Received)?,
                "updated_at":Utc::now().timestamp(),
            }
        };

        let tm = Utc::now() - Duration::minutes(1);

        let query = doc! {
            "node_name":node_name,
            "state":  to_variant_name(&DataState::SelectForSend)?,
            "direction":to_variant_name(&direction)?,
            "updated_at": {
                "$lt": tm.timestamp()
            },
        };

        self.data_col
            .update_many(query, update)
            .await
            .map(|r| r.modified_count)
            .anyhow()
    }

    async fn find_by_node_id(
        &self,
        node_name: &str,
        id: &str,
        direction: &Direction,
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
        state: &DataState,
    ) -> Result<Vec<DataRecord>> {
        self.data_col
            .find(doc! {"node_name":node_name, "state": to_variant_name(&state)?})
            .await?
            .try_collect()
            .await
            .anyhow()
    }

    async fn count(
        &self,
        node_name: &str,
        states: &[&DataState],
        direction: &Direction,
    ) -> Result<usize> {
        let states: Vec<&str> = states.iter().map(to_variant_name).try_collect()?;
        self.data_col
            .count_documents(doc! {
                "node_name":node_name,
                "state": doc! {"$in": states},
                "direction": to_variant_name(&direction)?
            })
            .await
            .map(|count| count as usize)
            .anyhow()
    }

    async fn update_state(
        &self,
        node_name: &str,
        id: &str,
        direction: &Direction,
        state: &DataState,
        sent: Option<Vec<&str>>,
    ) -> Result<()> {
        let mut update_fields = doc! {
            "state":  to_variant_name(&state)?,
            "updated_at":Utc::now().timestamp()
        };

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
