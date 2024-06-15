use super::graph::Graph;
use crate::core::{ComputeUnit, GID};
use crate::utils::IntoAnyhowResult;
use anyhow::{anyhow, Ok, Result};
use std::collections::HashMap;

pub struct Dag<ID>
where
    ID: GID,
{
    id: ID,
    name: String,
    nodes: HashMap<ID, ComputeUnit<ID>>,
    /// Store dependency relations.
    rely_graph: Graph<ID>,
}

impl<ID: GID> Dag<ID> {
    pub fn new() -> Self {
        Dag {
            id: ID::default(),
            name: String::new(),
            nodes: HashMap::new(),
            rely_graph: Graph::new(),
        }
    }

    //add_node add a compute unit to graph, if this unit alread exit, ignore it
    pub fn add_node(&mut self, node: ComputeUnit<ID>) -> &mut Self {
        if self.nodes.get(&node.id).is_none() {
            self.rely_graph.add_node(node.id.clone());
            self.nodes.insert(node.id.clone(), node);
        }
        self
    }

    //add_node add a compute unit to graph, if this unit alread exit, ignore it
    pub fn set_edge(&mut self, from: &ID, to: &ID) -> Result<&mut Self> {
        if self.nodes.get(from).is_none() {
            return Err(anyhow!("from node not exit"));
        }

        if self.nodes.get(to).is_none() {
            return Err(anyhow!("from node not exit"));
        }

        self.rely_graph.add_edge(from, to);
        Ok(self)
    }

    // from_json build graph from json string
    pub fn from_json<'a>(json: &'a str) -> Result<Self> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        let id: ID = value
            .get("id")
            .anyhow("id must exit")
            .map(|v| serde_json::from_value::<ID>(v.clone()))??;

        let dag_name: &str = value
            .get("name")
            .anyhow("name must exit")
            .map(|v| v.as_str().anyhow("name must be string"))??;

        let dag = value.get("dag").anyhow("dag must exit")?;

        let mut nodes = vec![];
        for node in dag.as_array().anyhow("dag must be a arrary")?.iter() {
            nodes.push(serde_json::from_value::<ComputeUnit<ID>>(node.clone())?);
        }

        let node_ids: Vec<ID> = nodes.iter().map(|node| node.id).collect();
        let mut rely_graph = Graph::with_nodes(node_ids.as_slice());
        for node in nodes.iter() {
            node.dependency.iter().for_each(|v| {
                rely_graph.add_edge(v, &node.id);
            });
        }

        let nodes_map: HashMap<ID, ComputeUnit<ID>> =
            nodes.into_iter().map(|node| (node.id, node)).collect();
        Ok(Dag {
            id: id,
            name: dag_name.to_string(),
            nodes: nodes_map,
            rely_graph: rely_graph,
        })
    }

    // is_validated do some check on the graph to ensure that all correct
    pub fn is_validated(&self) -> bool {
        todo!();
        //check id

        //must have channel except the end node
    }

    // iter immutable iterate over graph
    pub fn iter(&self) -> GraphIter<ID> {
        GraphIter {
            index: 0,
            data: &self.nodes,
            toped_graph: self.rely_graph.topo_sort(),
        }
    }

    // iter_mut mutable iterate over graph
    pub fn iter_mut(&mut self) -> GraphIterMut<ID> {
        GraphIterMut {
            index: 0,
            data: &mut self.nodes,
            toped_graph: self.rely_graph.topo_sort(),
        }
    }
}

pub struct GraphIter<'a, ID>
where
    ID: GID,
{
    index: usize,
    data: &'a HashMap<ID, ComputeUnit<ID>>,
    toped_graph: Vec<ID>,
}

impl<'a, ID: GID> Iterator for GraphIter<'a, ID> {
    type Item = &'a ComputeUnit<ID>;

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

pub struct GraphIterMut<'a, ID>
where
    ID: GID,
{
    index: usize,
    data: &'a mut HashMap<ID, ComputeUnit<ID>>,
    toped_graph: Vec<ID>,
}

impl<'a, ID: GID> Iterator for GraphIterMut<'a, ID> {
    type Item = &'a mut ComputeUnit<ID>;

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
                let node_ptr = node as *mut ComputeUnit<ID>;
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
    use uuid::Uuid;

    #[test]
    fn deserialize_from_str() {
        let json_str = r#"
{
  "name": "example",
  "version": "v1",
  "dag": [
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
            "bufsize",
            "1024"
          ],
          "image": "jiaozifs:"
        }
      }
    },
    {
      "id": "353fc5bf-697e-4221-8487-6ab91915e2a1",
      "name": "ComputeUnit2",
      "node_type": "ComputeUnit",
      "dependency": [
        "5c42b900-a87f-45e3-ba06-c40d94ad5ba2"
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
        let mut result = Dag::<Uuid>::from_json(json_str).unwrap();
        let node_names: Vec<_> = result.iter().map(|node| node.name.clone()).collect();
        assert_eq!(["ComputeUnit1", "ComputeUnit2"], node_names.as_slice());

        for (index, node) in result.iter_mut().enumerate() {
            node.name = "node".to_string() + &index.to_string();
        }

        let node_names: Vec<_> = result.iter().map(|node| node.name.clone()).collect();
        assert_eq!(["node0", "node1"], node_names.as_slice());
    }
}
