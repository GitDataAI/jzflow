mod graph;

use graph::Graph;
use crate::{
    core::ComputeUnit,
    utils::IntoAnyhowResult,
};
use anyhow::{
    anyhow,
    Ok,
    Result,
};
use std::collections::HashMap;

pub struct Dag {
    pub raw: String,
    pub name: String,
    nodes: HashMap<String, ComputeUnit>,
    /// Store dependency relations.
    rely_graph: Graph,
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

impl Dag {
    pub fn new() -> Self {
        Dag {
            raw: String::new(),
            name: String::new(),
            nodes: HashMap::new(),
            rely_graph: Graph::new(),
        }
    }

    //add_node add a compute unit to graph, if this unit alread exit, ignore it
    pub fn add_node(&mut self, node: ComputeUnit) -> &mut Self {
        if !self.nodes.contains_key(&node.name) {
            self.rely_graph.add_node(node.name.clone());
            self.nodes.insert(node.name.clone(), node);
        }
        self
    }

    //add_node add a compute unit to graph, if this unit alread exit, ignore it
    pub fn set_edge(&mut self, from: &str, to: &str) -> Result<&mut Self> {
        if !self.nodes.contains_key(from) {
            return Err(anyhow!("from node not exit"));
        }

        if !self.nodes.contains_key(to) {
            return Err(anyhow!("from node not exit"));
        }

        self.rely_graph.add_edge(from, to);
        Ok(self)
    }

    pub fn get_node(&self, node_id: &str) -> Option<&ComputeUnit> {
        self.nodes.get(node_id)
    }

    pub fn get_incomming_nodes(&self, node_id: &str) -> Vec<&str> {
        self.rely_graph.get_incoming_nodes(node_id)
    }

    pub fn get_outgoing_nodes(&self, node_id: &str) -> Vec<&str> {
        self.rely_graph.get_outgoing_nodes(node_id)
    }

    // from_json build graph from json string
    pub fn from_json(json: &str) -> Result<Self> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        let dag_name: &str = value
            .get("name")
            .anyhow("name must exit")
            .map(|v| v.as_str().anyhow("name must be string"))??;

        let dag = value.get("dag").anyhow("dag must exit")?;

        let mut nodes = vec![];
        for node in dag.as_array().anyhow("dag must be a arrary")?.iter() {
            nodes.push(serde_json::from_value::<ComputeUnit>(node.clone())?);
        }

        let node_ids: Vec<String> = nodes.iter().map(|node| node.name.clone()).collect();
        let mut rely_graph = Graph::with_nodes(&node_ids);
        for node in nodes.iter() {
            node.dependency.iter().for_each(|v| {
                rely_graph.add_edge(v, &node.name);
            });
        }

        let nodes_map: HashMap<String, ComputeUnit> = nodes
            .into_iter()
            .map(|node| (node.name.clone(), node))
            .collect();
        Ok(Dag {
            name: dag_name.to_string(),
            raw: json.to_string(),
            nodes: nodes_map,
            rely_graph,
        })
    }

    // is_validated do some check on the graph to ensure that all correct
    pub fn is_validated(&self) -> bool {
        todo!();
        //check id

        //must have channel except the end node
    }

    // iter immutable iterate over graph
    pub fn iter(&self) -> GraphIter {
        GraphIter {
            index: 0,
            data: &self.nodes,
            toped_graph: self.rely_graph.topo_sort(),
        }
    }

    // iter_mut mutable iterate over graph
    pub fn iter_mut(&mut self) -> GraphIterMut {
        GraphIterMut {
            index: 0,
            data: &mut self.nodes,
            toped_graph: self.rely_graph.topo_sort(),
        }
    }
}

pub struct GraphIter<'a> {
    index: usize,
    data: &'a HashMap<String, ComputeUnit>,
    toped_graph: Vec<String>,
}

impl<'a> Iterator for GraphIter<'a> {
    type Item = &'a ComputeUnit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.len() {
            let item = self.toped_graph[self.index].clone();
            self.index += 1;
            Some(self.data.get(&item).expect("node added in previous step"))
        } else {
            None
        }
    }
}

pub struct GraphIterMut<'a> {
    index: usize,
    data: &'a mut HashMap<String, ComputeUnit>,
    toped_graph: Vec<String>,
}

impl<'a> Iterator for GraphIterMut<'a> {
    type Item = &'a mut ComputeUnit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.len() {
            let item = self.toped_graph[self.index].clone();
            self.index += 1;
            let node = self
                .data
                .get_mut(&item)
                .expect("node added in previous step");
            unsafe {
                // SAFETY: We ensure no two mutable references to the same element are possible.
                let node_ptr = node as *mut ComputeUnit;
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
    fn deserialize_from_str() {
        let json_str = r#"
{
  "name": "example",
  "version": "v1",
  "dag": [
    {
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
            "bufsize",
            "1024"
          ],
          "image": "jiaozifs:"
        }
      }
    },
    {
      "name": "ComputeUnit2",
      "node_type": "ComputeUnit",
      "dependency": [
        "ComputeUnit1"
      ],
      "spec": {
        "cmd": [
          "ls"
        ],
        "image": ""
      }
    }
  ]
}
                "#;
        let mut result = Dag::from_json(json_str).unwrap();
        let node_names: Vec<_> = result.iter().map(|node| node.name.clone()).collect();
        assert_eq!(["ComputeUnit1", "ComputeUnit2"], node_names.as_slice());

        for (index, node) in result.iter_mut().enumerate() {
            node.name = "node".to_string() + &index.to_string();
        }

        let node_names: Vec<_> = result.iter().map(|node| node.name.clone()).collect();
        assert_eq!(["node0", "node1"], node_names.as_slice());
    }
}
