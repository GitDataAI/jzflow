use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    CoputeUnit,
    Channel,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
    pub input_output_type: i32,
    pub upstreams: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub trait GraphRepo {
    async fn insert_global_state(&self, state: Graph) -> Result<()>;
    async fn get_global_state(&self) -> Result<Graph>;
}

pub trait NodeRepo {
    async fn insert_node(&self, state: Node) -> Result<()>;
    async fn get_node_by_name(&self, name: &str) -> Result<Node>;
}
