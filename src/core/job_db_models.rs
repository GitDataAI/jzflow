use anyhow::Result;
use serde::{
    Deserialize,
    Serialize,
};

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
    pub up_nodes: Vec<String>,
    pub incoming_streams: Vec<String>,
    pub outgoing_streams: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// DataState use to represent databatch state
/// for incoming data in compute unit:  Received(compute data) -> Assigned(assigned to user
/// containerd) -> Processed(user containerd has processed)
///
/// for channel dataflow: Received(channel) -> Assigned(channel)->Sent(send to channel) ->
/// EndRecieved(compute unit)-> Clean(channel clean data)
///
/// for outgoing data flow of compute unit:  Received(compute unit) -> SelectForSend(compute unit)
/// -> PartialSent(compute unit)->Sent(compute unit)
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum DataState {
    Received,
    Assigned,
    Processed,
    SelectForSend,
    PartialSent,
    Sent,
    EndRecieved,
    Clean,
    Error,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Direction {
    In,
    Out,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataRecord {
    /// node's for compute unit its a name,  for channel its node_name+ "-channel"
    pub node_name: String,
    /// data id but not unique, becase the id in channel wiil transfer to compute unit
    pub id: String,

    pub size: u32,
    pub state: DataState,
    pub direction: Direction,
    pub sent: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub trait GraphRepo {
    fn insert_global_state(
        &self,
        graph: &Graph,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn get_global_state(&self) -> impl std::future::Future<Output = Result<Graph>> + Send;
}

pub trait NodeRepo {
    fn insert_node(&self, state: &Node) -> impl std::future::Future<Output = Result<()>> + Send;
    fn get_node_by_name(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<Node>> + Send;

    fn update_node_by_name(
        &self,
        name: &str,
        state: TrackerState,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub trait DataRepo {
    fn find_data_and_mark_state(
        &self,
        node_name: &str,
        direction: &Direction,
        state: &DataState,
    ) -> impl std::future::Future<Output = Result<Option<DataRecord>>> + Send;

    fn find_by_node_id(
        &self,
        node_name: &str,
        id: &str,
        direction: &Direction,
    ) -> impl std::future::Future<Output = Result<Option<DataRecord>>> + Send;

    fn revert_no_success_sent(
        &self,
        node_name: &str,
        direction: &Direction,
    ) -> impl std::future::Future<Output = Result<u64>> + Send;

    fn list_by_node_name_and_state(
        &self,
        node_name: &str,
        state: &DataState,
    ) -> impl std::future::Future<Output = Result<Vec<DataRecord>>> + Send;

    fn count(
        &self,
        node_name: &str,
        states: &[&DataState],
        direction: Option<&Direction>,
    ) -> impl std::future::Future<Output = Result<usize>> + Send;

    fn insert_new_path(
        &self,
        record: &DataRecord,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn update_state(
        &self,
        node_name: &str,
        id: &str,
        direction: &Direction,
        state: &DataState,
        sent: Option<Vec<&str>>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub trait JobDbRepo = GraphRepo + NodeRepo + DataRepo + Clone + Send + Sync + 'static;
