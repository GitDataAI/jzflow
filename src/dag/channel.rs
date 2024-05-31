use super::base::{BaseUnit, UnitType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelUnit {
    pub id: uuid::Uuid,
    pub name: String,

    pub(crate) dependency: Vec<Uuid>,
    pub(crate) node_type: UnitType,
}

impl BaseUnit for ChannelUnit {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
