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
    InComingFinish, //mean all incoming data was processed
    Finish,
}

impl TrackerState {
    pub fn is_end_state(&self) -> bool {
        match self {
            TrackerState::Init => false,
            TrackerState::Ready => false,
            TrackerState::Stop => false,
            TrackerState::Stopped => true,
            TrackerState::InComingFinish => false,
            TrackerState::Finish => true,
        }
    }
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
    CleanButKeepData,
    Clean,
    Error,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Direction {
    In,
    Out,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct DataFlag {
    pub is_keep_data: bool,
    pub is_transparent_data: bool,
}

pub const KEEP_DATA: u32 = 0b00000001;
pub const TRANSPARENT_DATA: u32 = 0b00000010;

impl DataFlag {
    pub fn to_bit(&self) -> u32 {
        let mut result = 0;
        if self.is_transparent_data {
            result |= TRANSPARENT_DATA;
        }
        if self.is_keep_data {
            result |= KEEP_DATA;
        }
        result
    }

    pub fn new_from_bit(bits: u32) -> Self {
        let mut flag = DataFlag::default();
        if bits & KEEP_DATA == KEEP_DATA {
            flag.is_keep_data = true;
        }
        if bits & TRANSPARENT_DATA == TRANSPARENT_DATA {
            flag.is_transparent_data = true;
        }
        flag
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataRecord {
    /// node's for compute unit its a name,  for channel its node_name+ "-channel"
    pub node_name: String,
    /// data id but not unique, becase the id in channel wiil transfer to compute unit
    pub id: String,
    pub priority: u8,
    pub flag: DataFlag,
    pub size: u32,
    pub state: DataState,
    pub direction: Direction,
    pub machine: String,
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

    fn is_all_node_finish(&self) -> impl std::future::Future<Output = Result<bool>> + Send;

    fn update_node_by_name(
        &self,
        name: &str,
        state: TrackerState,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn mark_incoming_finish(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub trait DataRepo {
    fn find_data_and_mark_state(
        &self,
        node_name: &str,
        direction: &Direction,
        include_transparent_data: bool,
        state: &DataState,
        machine_name: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_data_flag() {
        let data = DataFlag::new_from_bit(KEEP_DATA | TRANSPARENT_DATA);
        assert!(data.is_keep_data);
        assert!(data.is_transparent_data);

        let bit_value = data.to_bit();
        assert_eq!(bit_value, 3);
    }
}
