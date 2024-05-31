use super::base::{BaseNode, NodeType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Channel {
    pub id: uuid::Uuid,
    pub name: String,

    pub(crate) dependency: Vec<Uuid>,
    pub(crate) node_type: NodeType,
}

impl BaseNode for Channel {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
