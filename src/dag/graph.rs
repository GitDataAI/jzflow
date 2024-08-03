/*!
node Graph

# Graph stores dependency relations.

[`Graph`] represents a series of nodes with dependencies, and stored in an adjacency
list. It must be a directed acyclic graph, that is, the dependencies of the node
cannot form a loop, otherwise the engine will not be able to execute the node successfully.
It has some useful methods for building graphs, such as: adding edges, nodes, etc.
And the most important of which is the `topo_sort` function, which uses topological
sorting to generate the execution sequence of nodes.

# An example of a directed acyclic graph

node1 -→ node3 ---→ node6 ----
 |   ↗   ↓          ↓         ↘
 |  /   node5 ---→ node7 ---→ node9
 ↓ /      ↑          ↓         ↗
node2 -→ node4 ---→ node8 ----

The node execution sequence can be as follows:
node1->node2->node3->node4->node5->node6->node7->node8->node9

*/

use std::{
    collections::{
        HashMap,
        HashSet,
        VecDeque,
    },
    vec,
};

#[derive(Debug, Clone)]
/// Graph Struct
pub(crate) struct Graph {
    nodes: HashSet<String>,
    nodes_count: usize,
    /// Adjacency list of graph (stored as a vector of vector of indices)
    adj: HashMap<String, Vec<String>>,
    /// Node's in_degree, used for topological sort
    in_degree: HashMap<String, Vec<String>>,
}

impl Graph {
    /// Allocate an empty graph
    pub(crate) fn new() -> Self {
        Graph {
            nodes: HashSet::new(),
            nodes_count: 0,
            adj: HashMap::new(),
            in_degree: HashMap::new(),
        }
    }

    /// init with a batch of nodes
    pub(crate) fn with_nodes(ids: &[String]) -> Self {
        let mut graph = Graph::new();
        ids.iter().for_each(|id| {
            graph.add_node(id.clone());
        });
        graph
    }

    // Append a node in graph
    pub(crate) fn add_node(&mut self, id: String) {
        if self.nodes.contains(&id) {
            return;
        }

        self.nodes_count += 1;
        self.nodes.insert(id.clone());
        self.adj.insert(id.clone(), vec![]);
        self.in_degree.insert(id.clone(), vec![]);
    }

    /// Add an edge into the graph.
    /// Above operation adds a arrow from node 0 to node 1,
    /// which means node 0 shall be executed before node 1.
    pub(crate) fn add_edge(&mut self, from: &str, to: &str) {
        match self.adj.get_mut(from) {
            Some(v) => {
                if !v.contains(&from.to_string()) {
                    v.push(to.to_string());
                }
            }
            None => {
                self.adj.insert(from.to_string(), vec![to.to_string()]);
            }
        }

        match self.in_degree.get_mut(to) {
            Some(v) => {
                if !v.contains(&to.to_string()) {
                    v.push(from.to_string());
                }
            }
            None => {
                self.in_degree
                    .insert(to.to_string(), vec![from.to_string()]);
            }
        }
    }

    /// Do topo sort in graph, returns a possible execution sequence if DAG.
    /// This operation will judge whether graph is a DAG or not,
    /// returns Some(Possible Sequence) if yes, and None if no.
    ///
    ///
    /// **Note**: this function can only be called after graph's initialization (add nodes and
    /// edges, etc.) is done.
    ///
    /// # Principle
    /// Reference: [Topological Sorting](https://www.jianshu.com/p/b59db381561a)
    ///
    /// 1. For a graph g, we record the in-degree of every node.
    ///
    /// 2. Each time we start from a node with zero in-degree, name it N0, and N0 can be executed
    ///    since it has no dependency.
    ///
    /// 3. And then we decrease the in-degree of N0's children (those nodes depend on N0), this
    ///    would create some new zero in-degree nodes.
    ///
    /// 4. Just repeat step 2, 3 until no more zero degree nodes can be generated. If all nodes have
    ///    been executed, then it's a DAG, or there must be a loop in the graph.
    pub(crate) fn topo_sort(&self) -> Vec<String> {
        let mut queue = self
            .in_degree
            .iter()
            .filter_map(|(cur, ins)| if ins.len() == 0 { Some(cur) } else { None })
            .collect::<VecDeque<_>>();

        let mut in_degree = self.in_degree.clone();
        let mut sequence: Vec<String> = Vec::with_capacity(self.nodes_count);

        while let Some(v) = queue.pop_front() {
            sequence.push(v.clone());

            for id in self.adj[v].iter() {
                let ins = in_degree.get_mut(id).expect("in node must exit");
                ins.retain(|p| v != p);
                if ins.is_empty() {
                    queue.push_back(id)
                }
            }
        }

        sequence
    }

    /// Get the out degree of a node.
    pub(crate) fn get_out_degree(&self, id: &str) -> usize {
        match self.adj.get(id) {
            Some(id) => id.len(),
            None => 0,
        }
    }

    /// Get the in_comming nodes.
    pub(crate) fn get_in_degree(&self, id: &str) -> usize {
        match self.in_degree.get(id) {
            Some(id) => id.len(),
            None => 0,
        }
    }

    /// Get the out nodes of a node.
    pub(crate) fn get_incoming_nodes(&self, id: &str) -> Vec<&str> {
        match self.in_degree.get(id) {
            Some(id) => id.iter().map(|v| v.as_str()).collect(),
            None => vec![],
        }
    }

    /// Get the out nodes of a node.
    pub(crate) fn get_outgoing_nodes(&self, id: &str) -> Vec<&str> {
        match self.adj.get(id) {
            Some(id) => id.iter().map(|v| v.as_str()).collect(),
            None => vec![],
        }
    }

    /// Get all the successors of a node (direct or indirect).
    /// This function will return a vector of indices of successors (including itself).
    pub(crate) fn get_node_successors(&self, id: &str) -> Vec<String> {
        match self.adj.get(id) {
            Some(outs) => {
                // initialize a vector to store successors with max possible size
                let mut successors: Vec<String> = Vec::with_capacity(outs.len());

                // create a visited array to avoid visiting a node more than once
                let mut visited = HashSet::new();
                let mut stack = vec![id];

                visited.insert(id);
                successors.push(id.to_string());
                // while the queue is not empty
                while !stack.is_empty() {
                    let v = stack.remove(0).to_string();
                    if let Some(out_degress) = self.adj.get(&v) {
                        for id in out_degress.iter() {
                            if !visited.contains(id.as_str()) {
                                // if not visited, mark it as visited and collect it
                                visited.insert(id);
                                successors.push(id.to_string());
                                stack.push(id);
                            }
                        }
                    }
                }
                successors
            }
            // If node not found, return empty vector
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        // "a" => "b"
        // "a" => "c"
        // "b" => "d"
        // "b" => "d"
        let a = "a".to_string();
        let b = "b".to_string();
        let c = "c".to_string();
        let d = "d".to_string();

        let mut graph = Graph::with_nodes([a.clone(), b.clone(), c.clone(), d.clone()].as_slice());
        graph.add_edge(&a, &b);
        graph.add_edge(&a, &c);
        graph.add_edge(&b, &d);
        graph.add_edge(&c, &d);

        let topo_ids = graph.topo_sort();
        let sequence: Vec<&str> = topo_ids.iter().map(|id| id.as_str()).collect();
        assert_eq!(["a", "b", "c", "d"], sequence.as_slice());
    }

    #[test]
    fn test_mulity_path() {
        // "a" => "b"
        // "a" => "c"
        // "b" => "d"
        // "b" => "d"
        let a = "a".to_string();
        let b = "b".to_string();
        let c = "c".to_string();
        let d = "d".to_string();
        let e = "e".to_string();
        let f = "f".to_string();
        let g = "g".to_string();

        let mut graph = Graph::with_nodes(
            [
                a.clone(),
                b.clone(),
                c.clone(),
                d.clone(),
                e.clone(),
                f.clone(),
                g.clone(),
            ]
            .as_slice(),
        );
        graph.add_edge(&a, &b);
        graph.add_edge(&a, &c);
        graph.add_edge(&b, &d);
        graph.add_edge(&c, &d);
        graph.add_edge(&a, &e);
        graph.add_edge(&e, &f);
        graph.add_edge(&f, &g);
        graph.add_edge(&d, &g);

        let topo_ids = graph.topo_sort();
        let sequence: Vec<&str> = topo_ids.iter().map(|id| id.as_str()).collect();
        assert_eq!(["a", "b", "c", "e", "d", "f", "g"], sequence.as_slice());
    }

    #[test]
    fn test_get_node_successors() {
        // "a" => "b"
        // "a" => "c"
        // "b" => "d"
        // "b" => "d"
        let a = "a".to_string();
        let b = "b".to_string();
        let c = "c".to_string();
        let d = "d".to_string();
        let e = "e".to_string();
        let f = "f".to_string();
        let g = "g".to_string();

        let mut graph = Graph::with_nodes(
            [
                a.clone(),
                b.clone(),
                c.clone(),
                d.clone(),
                e.clone(),
                f.clone(),
                g.clone(),
            ]
            .as_slice(),
        );
        graph.add_edge(&a, &b);
        graph.add_edge(&a, &c);
        graph.add_edge(&b, &d);
        graph.add_edge(&c, &d);
        graph.add_edge(&a, &e);
        graph.add_edge(&e, &f);
        graph.add_edge(&f, &g);
        graph.add_edge(&d, &g);

        let sequence = graph.get_node_successors(&c);
        let sequence: Vec<&str> = sequence.iter().map(|id| id.as_str()).collect();
        assert_eq!(["c", "d", "g"], sequence.as_slice());

        assert_eq!(0, graph.get_in_degree(&a));
        assert_eq!(1, graph.get_in_degree(&b));
        assert_eq!(1, graph.get_in_degree(&c));
        assert_eq!(2, graph.get_in_degree(&d));
        assert_eq!(1, graph.get_in_degree(&e));
        assert_eq!(1, graph.get_in_degree(&f));
        assert_eq!(2, graph.get_in_degree(&g));

        assert_eq!(3, graph.get_out_degree(&a));
        assert_eq!(1, graph.get_out_degree(&b));
        assert_eq!(1, graph.get_out_degree(&c));
        assert_eq!(1, graph.get_out_degree(&d));
        assert_eq!(1, graph.get_out_degree(&e));
        assert_eq!(1, graph.get_out_degree(&f));
        assert_eq!(0, graph.get_out_degree(&g));
    }
}
