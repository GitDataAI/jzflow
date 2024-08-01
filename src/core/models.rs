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

/// DataState use to represent databatch state
/// for incoming data:  Received(channel receive data) -> Assigned(assigned to user containerd) -> Processed(user containerd has processed)
/// for outgoing data:  Received(user container product) -> Sent(send to channel)

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum DataState {
    Received,
    Assigned,
    Processed,
    Sent,
    Error,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Direction {
    In,
    Out,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataRecord {
    pub node_name: String,
    pub id: String,
    pub size: u32,
    pub state: DataState,
    pub direction: Direction,
}

pub trait DBConfig {
    fn connection_string(&self) -> &str;
}

pub trait GraphRepo {
    fn insert_global_state(
        &self,
        state: Graph,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn get_global_state(&self) -> impl std::future::Future<Output = Result<Graph>> + Send;
}

pub trait NodeRepo {
    fn insert_node(&self, state: Node) -> impl std::future::Future<Output = Result<()>> + Send;
    fn get_node_by_name(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<Node>> + Send;
}

pub trait DataRepo {
    fn find_and_assign_input_data(
        &self,
        node_name: &str,
    ) -> impl std::future::Future<Output = Result<Option<DataRecord>>> + Send;

    fn find_and_sent_output_data(
        &self,
        node_name: &str,
    ) -> impl std::future::Future<Output = Result<Option<DataRecord>>> + Send;

    fn insert_new_path(
        &self,
        record: &DataRecord,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn update_state(
        &self,
        node_name: &str,
        id: &str,
        state: DataState,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub trait DbRepo = GraphRepo + NodeRepo + DataRepo + Clone + Send + Sync + 'static;
