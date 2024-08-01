use super::MachineSpec;
use serde::{Deserialize, Serialize};

// DataPoint use to definite data transfer channel
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataPoint {
    pub spec: MachineSpec,
}

// ComputeUnit used to define logic for data generation, transformer, ouput.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComputeUnit {
    pub name: String,

    pub spec: MachineSpec,

    pub(crate) dependency: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let json_str = r#"
      {
        "name": "ComputeUnit1",
        "dependency": [
          
        ],
        "spec": {
          "cmd": [
            "ls"
          ],
          "image": ""
        }
      }
              "#
        .to_owned();
        serde_json::from_str::<ComputeUnit>(&json_str).unwrap();
    }
}
