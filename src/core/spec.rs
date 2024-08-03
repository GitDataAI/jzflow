use serde::{
    Deserialize,
    Serialize,
};

// MachineSpec container information for deploy and running in cloud
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MachineSpec {
    pub image: String,
    pub replicas: u32,
    pub cmd: Vec<String>,
}
