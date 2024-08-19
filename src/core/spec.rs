use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, PartialEq, Deserialize, Debug, Clone, Default)]
pub enum CacheType {
    #[default]
    Disk,
    Memory,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageOptions {
    #[serde(default = "default_storage_class_name")]
    pub class_name: String,

    #[serde(default = "default_capacity")]
    pub capacity: String,
}

impl Default for StorageOptions {
    fn default() -> Self {
        Self {
            class_name: default_storage_class_name(),
            capacity: default_capacity(),
        }
    }
}

fn default_storage_class_name() -> String {
    "jz-flow-fs".to_string()
}

fn default_capacity() -> String {
    "1Gi".to_string()
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

    #[serde(default)]
    pub storage: StorageOptions,
}

fn default_replicas() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let spec: MachineSpec = serde_json::from_str("{}").unwrap();
        assert_eq!(spec.replicas, 1);
        assert_eq!(spec.cache_type, CacheType::Disk);
        assert_eq!(spec.storage.class_name, default_storage_class_name());
        assert_eq!(spec.storage.capacity, default_capacity());
    }
}
