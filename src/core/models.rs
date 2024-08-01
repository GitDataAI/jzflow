use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    CoputeUnit,
    Channel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrackerState {
    Init,
    Ready,
    Stop,
    Stopped,
    Finish,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Graph {
    pub graph_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub node_name: String,
    pub state: TrackerState,
    pub node_type: NodeType,
    pub upstreams: Vec<String>,
    pub downstreams: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize,Debug, PartialEq)]
pub enum DataState {
    Received,
    Assigned,
    Processed,
    Sent,
}

#[derive(Serialize, Deserialize,Debug, PartialEq)]
pub enum Direction {
    In,
    Out,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataRecord {
    pub node_name: String,
    pub id: String,
    pub state: DataState,
    pub direction: Direction,
}

pub trait DBConfig {
    fn connection_string(&self) -> &str;
}

pub trait GraphRepo {
    async fn insert_global_state(&self, state: Graph) -> Result<()>;
    async fn get_global_state(&self) -> Result<Graph>;
}

pub trait NodeRepo {
    async fn insert_node(&self, state: Node) -> Result<()>;
    async fn get_node_by_name(&self, name: &str) -> Result<Node>;
}

pub trait DataRepo {
    async fn get_avaiable_data(&self, node_name: &str)-> Result<Option<String>>;
    async fn insert_new_path(&self, record: &DataRecord)-> Result<()>;
    async fn update_state(&self, node_name: &str, id: &str, state: DataState) ->Result<()>;
}

pub trait DbRepo = GraphRepo + NodeRepo + DataRepo+Clone + Send + Sync + 'static;

