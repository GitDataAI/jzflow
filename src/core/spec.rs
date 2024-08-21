use anyhow::anyhow;
use serde::{
    Deserialize,
    Serialize,
};
use std::str::FromStr;

#[derive(Serialize, PartialEq, Deserialize, Debug, Clone, Default)]
pub enum CacheType {
    #[default]
    Disk,
    Memory,
}

#[derive(Serialize, Deserialize, Debug, Clone)]

pub enum AccessMode {
    ReadWriteMany,
    ReadWriteOnce,
}

impl FromStr for AccessMode {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<AccessMode, Self::Err> {
        match input {
            "ReadWriteMany" => Ok(AccessMode::ReadWriteMany),
            "ReadWriteOnce" => Ok(AccessMode::ReadWriteOnce),
            _ => Err(anyhow!("unsupport acess mode {input}")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageOptions {
    pub class_name: Option<String>,
    pub capacity: Option<String>,
    pub access_mode: Option<AccessMode>,
}

impl Default for StorageOptions {
    #[allow(unconditional_recursion)]
    fn default() -> Self {
        Self {
            class_name: None,
            capacity: None,
            access_mode: None,
        }
    }
}

// MachineSpec container information for deploy and running in cloud
#[derive(Serialize, Default, Deserialize, Debug, Clone)]
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
        assert!(spec.storage.class_name.is_none());
        assert!(spec.storage.capacity.is_none());
    }
}
