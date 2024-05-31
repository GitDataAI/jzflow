use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum UnitType {
    ChannelUnit,
    ComputeUnit,
}
pub trait BaseUnit {
    fn id(&self) -> Uuid;
    fn name(&self) -> String;
}
