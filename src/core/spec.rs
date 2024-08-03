use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CacheType {
    Disk,
    Memory,
}

impl Default for CacheType {
    fn default() -> Self {
        CacheType::Disk
    }
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
