use super::{MachineSpec, GID};
use serde::{Deserialize, Serialize};

// Channel use to definite data transfer channel
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Channel {
    pub spec: MachineSpec,
}

// ComputeUnit used to define logic for data generation, transformer, ouput.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComputeUnit<ID>
where
    ID: GID,
{
    #[serde(bound(deserialize = ""))]
    pub id: ID,

    pub name: String,

    pub spec: MachineSpec,

    pub channel: Option<Channel>,

    #[serde(bound(deserialize = ""))]
    pub(crate) dependency: Vec<ID>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_from_str() {
        let json_str = r#"
      {
        "id": "5c42b900-a87f-45e3-ba06-c40d94ad5ba2",
        "name": "ComputeUnit1",
        "dependency": [
          
        ],
        "spec": {
          "cmd": [
            "ls"
          ],
          "image": ""
        },
        "channel": {
          "spec": {
            "cmd": [
              "ls"
            ],
            "image": ""
          }
        }
      }
              "#
        .to_owned();
        serde_json::from_str::<ComputeUnit<Uuid>>(&json_str).unwrap();
    }
}
