use crate::{
    core::db::{
        DataRecord,
        DataState,
        Direction,
        Graph,
        Node,
        Repo,
        TrackerState,
    },
    utils::StdIntoAnyhowResult,
};
use chrono::{
    Duration,
    Utc,
};
use futures::TryStreamExt;
use mongodb::{
    bson::{
        doc,
        Document,
    },
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
    pub async fn new(connectstring: &str) -> anyhow::Result<Self> {
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

        async fn create_index<T>(
            collection: &Collection<T>,
            keys: Document,
            name: &str,
            unique: bool,
        ) -> anyhow::Result<()>
        where
            T: Send + Sync,
        {
            let idx_opts = IndexOptions::builder()
                .name(name.to_owned())
                .unique(unique)
                .build();
            let index = IndexModel::builder().keys(keys).options(idx_opts).build();

            if let Err(err) = collection.create_index(index).await {
                match *err.kind {
                    ErrorKind::Command(ref command_error) if command_error.code == 85 => Ok(()),
                    err => Err(anyhow::anyhow!("create index error {err}")),
                }
            } else {
                Ok(())
            }
        }

        // Create index for nodes
        create_index(
            &node_col,
            doc! { "node_name": 1 },
            "idx_node_name_unique",
            true,
        )
        .await?;

        // Create index for data
        create_index(&data_col, doc! { "created_at": 1 }, "idx_created_at", false).await?;

        create_index(
            &data_col,
            doc! { "node_name": 1, "state": 1, "direction": 1 },
            "idx_node_name_state_direction",
            false,
        )
        .await?;

        create_index(
            &data_col,
            doc! { "node_name": 1, "id": 1, "direction": 1 },
            "idx_node_name_id_direction",
            false,
        )
        .await?;

        create_index(
            &data_col,
            doc! { "node_name": 1, "id": 1, "direction": 1, "data.is_transparent_data": 1 },
            "idx_node_name_id_direction_transparent_data",
            false,
        )
        .await?;

        Ok(MongoRunDbRepo {
            graph_col,
            node_col,
            data_col,
        })
    }

    pub async fn drop(connectstring: &str) -> anyhow::Result<()> {
        let options = ClientOptions::parse(connectstring).await?;
        let database = options
            .default_database
            .as_ref()
            .expect("set db name in url")
            .clone();
        let client = Client::with_options(options)?;
        client.database(database.as_str()).drop().await.anyhow()
    }
}

impl Repo for MongoRunDbRepo {
    async fn insert_global_state(&self, graph: &Graph) -> anyhow::Result<()> {
        self.graph_col.insert_one(graph).await.map(|_| ()).anyhow()
    }

    async fn get_global_state(&self) -> anyhow::Result<Graph> {
        match self.graph_col.find_one(doc! {}).await {
            Ok(None) => Err(anyhow::anyhow!("global state not exit")),
            Ok(Some(val)) => Ok(val),
            Err(err) => Err(err.into()),
        }
    }
    async fn insert_node(&self, state: &Node) -> anyhow::Result<()> {
        self.node_col.insert_one(state).await.map(|_| ()).anyhow()
    }

    async fn get_node_by_name(&self, name: &str) -> anyhow::Result<Node> {
        match self.node_col.find_one(doc! {"node_name":name}).await {
            Ok(None) => Err(anyhow::anyhow!("node not exit")),
            Ok(Some(val)) => Ok(val),
            Err(err) => Err(err.into()),
        }
    }

    async fn update_node_by_name(&self, name: &str, state: TrackerState) -> anyhow::Result<()> {
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

    async fn mark_incoming_finish(&self, name: &str) -> anyhow::Result<()> {
        let update = doc! {
            "$set": {
                "state": to_variant_name(&TrackerState::InComingFinish)?,
                "updated_at":Utc::now().timestamp(),
            },
        };

        //provent override finish state
        self.node_col
            .update_one(
                doc! {
                    "node_name":name,
                    "state": {
                        "$ne": to_variant_name(&TrackerState::Finish)?
                    },
                },
                update,
            )
            .await
            .map(|_| ())
            .anyhow()
    }

    async fn is_all_node_finish(&self) -> anyhow::Result<bool> {
        Ok(self
            .node_col
            .count_documents(doc! {"state": {"$ne": to_variant_name(&TrackerState::Finish)?}})
            .await?
            == 0)
    }
    async fn find_data_and_mark_state(
        &self,
        node_name: &str,
        direction: &Direction,
        include_transparent_data: bool,
        state: &DataState,
        machine_name: Option<String>,
    ) -> anyhow::Result<Option<DataRecord>> {
        let mut set = doc! {
            "state": to_variant_name(&state)?,
            "updated_at":Utc::now().timestamp(),
        };
        if let Some(machine_name) = machine_name {
            set.insert("machine", machine_name);
        }

        let update = doc! {"$set":set};

        let mut query = doc! {
            "node_name":node_name,
            "state": doc! {"$in": ["Received","PartialSent"]},
            "direction":to_variant_name(&direction)?,
        };

        if !include_transparent_data {
            query.insert("flag.is_transparent_data", false);
        }

        self.data_col
            .find_one_and_update(query, update)
            .sort(doc! { "priority": 1 })
            .await
            .anyhow()
    }

    async fn revert_no_success_sent(
        &self,
        node_name: &str,
        direction: &Direction,
    ) -> anyhow::Result<u64> {
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
    ) -> anyhow::Result<Option<DataRecord>> {
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
    ) -> anyhow::Result<Vec<DataRecord>> {
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
        direction: Option<&Direction>,
    ) -> anyhow::Result<usize> {
        let mut query = doc! {"node_name":node_name};
        if !states.is_empty() {
            let states: Vec<&str> = states.iter().map(to_variant_name).try_collect()?;
            query.insert("state", doc! {"$in": states});
        }
        if let Some(direction) = direction {
            query.insert("direction", to_variant_name(&direction)?);
        }

        self.data_col
            .count_documents(query)
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
    ) -> anyhow::Result<()> {
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

    async fn insert_new_path(&self, record: &DataRecord) -> anyhow::Result<()> {
        self.data_col.insert_one(record).await.map(|_| ()).anyhow()
    }
}

