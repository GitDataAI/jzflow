use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum NodeType {
    Channel,
    ComputeUnit,
}
pub trait BaseNode {
    fn id(&self) -> Uuid;
    fn name(&self) -> String;
}
