use anyhow::{Ok, Result};
use std::collections::HashMap;
use uuid::Uuid;

use crate::utils::IntoAnyhowResult;

use super::base::{BaseUnit, UnitType};
use super::channel::ChannelUnit;
use super::cnode::ComputeUnit;
use super::graph::Graph;

pub enum Unit {
    CNode(Box<ComputeUnit>),
    Channel(Box<ChannelUnit>),
}

impl BaseUnit for Unit {
    fn id(&self) -> Uuid {
        match self {
            Unit::CNode(node) => node.id(),
            Unit::Channel(channel) => channel.id(),
        }
    }

    fn name(&self) -> String {
        match self {
            Unit::CNode(node) => node.name(),
            Unit::Channel(channel) => channel.name(),
        }
    }
}

pub struct Dag {
    name: String,
    nodes: HashMap<Uuid, Unit>,
    topoed_graph: Vec<Uuid>,
    /// Store dependency relations.
    rely_graph: Graph<Uuid>,
}

impl Dag {
    pub fn new() -> Self {
        Dag {
            name: String::new(),
            nodes: HashMap::new(),
            rely_graph: Graph::new(),
            topoed_graph: vec![],
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        // let reader = Cursor::new(json);
        //  let mut deserializer = Deserializer::from_reader(reader);
        let value: serde_json::Value = serde_json::from_str(json)?;
        let dag_name: &str = value
            .get("name")
            .anyhow("name must exit")
            .map(|v| v.as_str().anyhow("name must be string"))??;
        let dag = value.get("dag").anyhow("dag m ust exit")?;
        let nodes = dag
            .as_array()
            .anyhow("dag must be a arrary")?
            .iter()
            .map(|node_str| {
                let type_value = node_str.get("node_type").anyhow("node_type must exit")?;
                let node_type: UnitType = serde_json::from_value(type_value.clone())?;
                Ok(match node_type {
                    UnitType::ComputeUnit => {
                        Unit::CNode(Box::new(serde_json::from_value(node_str.clone())?))
                    }
                    UnitType::ChannelUnit => {
                        Unit::Channel(Box::new(serde_json::from_value(node_str.clone())?))
                    }
                })
            })
            .collect::<Result<Vec<Unit>, _>>()?;

        let node_ids: Vec<Uuid> = nodes.iter().map(|node| node.id()).collect();
        let mut rely_graph = Graph::with_nodes(node_ids.as_slice());
        for node in nodes.iter() {
            match node {
                Unit::CNode(compute_unit) => {
                    compute_unit.dependency.iter().for_each(|v| {
                        rely_graph.add_edge(*v, compute_unit.id());
                    });
                }
                Unit::Channel(channel_unit) => {
                    channel_unit.dependency.iter().for_each(|v| {
                        rely_graph.add_edge(*v, channel_unit.id());
                    });
                }
            }
        }

        let nodes_map: HashMap<Uuid, Unit> =
            nodes.into_iter().map(|node| (node.id(), node)).collect();
        Ok(Dag {
            name: dag_name.to_string(),
            nodes: nodes_map,
            topoed_graph: rely_graph.topo_sort(),
            rely_graph: rely_graph,
        })
    }

    pub fn iter(&self) -> GraphIter {
        GraphIter {
            index: 0,
            data: &self.nodes,
            toped_graph: &self.topoed_graph,
        }
    }

    pub fn iter_mut(&mut self) -> GraphIterMut {
        GraphIterMut {
            index: 0,
            data: &mut self.nodes,
            toped_graph: &self.topoed_graph,
        }
    }
}

pub struct GraphIter<'a> {
    index: usize,
    data: &'a HashMap<Uuid, Unit>,
    toped_graph: &'a Vec<Uuid>,
}

impl<'a> Iterator for GraphIter<'a> {
    type Item = &'a Unit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.len() {
            let item = self.toped_graph[self.index];
            self.index += 1;
            Some(self.data.get(&item).expect("node added in previous step"))
        } else {
            None
        }
    }
}

pub struct GraphIterMut<'a> {
    index: usize,
    data: &'a mut HashMap<Uuid, Unit>,
    toped_graph: &'a Vec<Uuid>,
}

impl<'a> Iterator for GraphIterMut<'a> {
    type Item = &'a mut Unit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.len() {
            let item = self.toped_graph[self.index];
            self.index += 1;
            let node = self
                .data
                .get_mut(&item)
                .expect("node added in previous step");
            unsafe {
                // SAFETY: We ensure no two mutable references to the same element are possible.
                let node_ptr = node as *mut Unit;
                Some(&mut *node_ptr)
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let json_str = r#"
        {
            "name":"example",
            "dag": [
                {
                    "id":"5c42b900-a87f-45e3-ba06-c40d94ad5ba2",
                    "name":"ComputeUnit1",
                    "node_type": "ComputeUnit",
                    "dependency": [],
                    "cmd": ["ls"],
                    "image":""
                },
                {
                    "id":"1193c01b-9847-4660-9ea1-34b66f7847f4",
                    "name":"Channel2",
                    "node_type": "ChannelUnit",
                    "dependency": ["5c42b900-a87f-45e3-ba06-c40d94ad5ba2"],
                    "image":""
                },
                {
                    "id":"353fc5bf-697e-4221-8487-6ab91915e2a1",
                    "name":"ComputeUnit3",
                    "node_type": "ComputeUnit",
                    "dependency": ["1193c01b-9847-4660-9ea1-34b66f7847f4"],
                    "cmd": ["ls"],
                    "image":""
                }
            ]
        }
    "#;
        let mut result = Dag::from_json(json_str).unwrap();
        let node_names: Vec<_> = result.iter().map(|node| node.name()).collect();
        assert_eq!(
            ["ComputeUnit1", "Channel2", "ComputeUnit3"],
            node_names.as_slice()
        );

        for (index, node) in result.iter_mut().enumerate() {
            match node {
                Unit::CNode(v) => {
                    v.name = "node".to_string() + &index.to_string();
                }
                Unit::Channel(v) => {
                    v.name = "channel".to_string() + &index.to_string();
                }
            }
        }

        let node_names: Vec<_> = result.iter().map(|node| node.name()).collect();
        assert_eq!(["node0", "channel1", "node2"], node_names.as_slice());
    }
}
