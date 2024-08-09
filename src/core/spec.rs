use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[derive(Default)]
pub enum CacheType {
    #[default]
    Disk,
    Memory,
}


// MachineSpec container information for deploy and running in cloud
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct MachineSpec {
    #[serde(default)]
    pub image: String,

    #[serde(default)]
    pub cache_type: CacheType,

    #[serde(default = "default_replicas")]
    pub replicas: u32,

    #[serde(default)]
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,
}

fn default_replicas() -> u32 {
    1
}
