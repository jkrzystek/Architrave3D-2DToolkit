use std::collections::{HashMap, HashSet, VecDeque};

use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use toolkit_core::{NodeId, ToolkitError, ToolkitResult};

use crate::node::NodeInstance;

/// Edge weight storing which output port connects to which input port.
#[derive(Clone, Debug)]
struct Connection {
    from_port: usize,
    to_port: usize,
}

/// Directed acyclic graph of `NodeInstance`s with connection tracking and
/// dirty-flag propagation.
pub struct NodeGraph {
    graph: StableDiGraph<NodeInstance, Connection>,
    id_to_index: HashMap<NodeId, NodeIndex>,
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::new(),
            id_to_index: HashMap::new(),
        }
    }

    /// Add a new node to the graph. Returns its `NodeId`.
    pub fn add_node(&mut self, template_name: String, position: (f32, f32)) -> NodeId {
        let id = NodeId::new();
        let instance = NodeInstance::new(id, template_name, position);
        let idx = self.graph.add_node(instance);
        self.id_to_index.insert(id, idx);
        id
    }

    /// Remove a node and all its connections. Returns `true` if the node existed.
    pub fn remove_node(&mut self, id: NodeId) -> bool {
        if let Some(idx) = self.id_to_index.remove(&id) {
            self.graph.remove_node(idx);
            true
        } else {
            false
        }
    }

    /// Connect an output port of `from_node` to an input port of `to_node`.
    /// Returns an error if the connection would create a cycle.
    pub fn connect(
        &mut self,
        from_node: NodeId,
        from_port: usize,
        to_node: NodeId,
        to_port: usize,
    ) -> ToolkitResult<()> {
        let from_idx = self
            .id_to_index
            .get(&from_node)
            .copied()
            .ok_or_else(|| ToolkitError::Custom(format!("Source node {from_node} not found")))?;
        let to_idx = self
            .id_to_index
            .get(&to_node)
            .copied()
            .ok_or_else(|| ToolkitError::Custom(format!("Target node {to_node} not found")))?;

        // Self-loops are always cycles.
        if from_idx == to_idx {
            return Err(ToolkitError::Custom(
                "Connection would create a cycle".into(),
            ));
        }

        // Cycle detection: if to_node can already reach from_node, adding this
        // edge would close a cycle.
        if self.can_reach(to_idx, from_idx) {
            return Err(ToolkitError::Custom(
                "Connection would create a cycle".into(),
            ));
        }

        self.graph.add_edge(
            from_idx,
            to_idx,
            Connection {
                from_port,
                to_port,
            },
        );

        // Mark the target (and its downstream) dirty.
        self.mark_dirty(to_node);

        Ok(())
    }

    /// Remove a specific port-level connection. Returns `true` if it existed.
    pub fn disconnect(
        &mut self,
        from_node: NodeId,
        from_port: usize,
        to_node: NodeId,
        to_port: usize,
    ) -> bool {
        let from_idx = match self.id_to_index.get(&from_node) {
            Some(idx) => *idx,
            None => return false,
        };
        let to_idx = match self.id_to_index.get(&to_node) {
            Some(idx) => *idx,
            None => return false,
        };

        let edge = self
            .graph
            .edges_connecting(from_idx, to_idx)
            .find(|e| e.weight().from_port == from_port && e.weight().to_port == to_port)
            .map(|e| e.id());

        if let Some(edge_id) = edge {
            self.graph.remove_edge(edge_id);
            true
        } else {
            false
        }
    }

    /// Mark a node and all its downstream dependents as dirty using BFS.
    pub fn mark_dirty(&mut self, id: NodeId) {
        let start_idx = match self.id_to_index.get(&id) {
            Some(idx) => *idx,
            None => return,
        };

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        queue.push_back(start_idx);
        visited.insert(start_idx);

        while let Some(idx) = queue.pop_front() {
            if let Some(node) = self.graph.node_weight_mut(idx) {
                node.dirty = true;
            }

            for neighbor in self
                .graph
                .neighbors_directed(idx, Direction::Outgoing)
                .collect::<Vec<_>>()
            {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    /// Returns all node IDs in a valid topological evaluation order.
    pub fn topological_order(&self) -> Vec<NodeId> {
        // Kahn's algorithm
        let mut in_degree: HashMap<NodeIndex, usize> = HashMap::new();
        for idx in self.graph.node_indices() {
            in_degree.insert(
                idx,
                self.graph
                    .neighbors_directed(idx, Direction::Incoming)
                    .count(),
            );
        }

        let mut queue: VecDeque<NodeIndex> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&idx, _)| idx)
            .collect();

        // Sort the initial queue for deterministic ordering.
        let mut sorted_queue: Vec<NodeIndex> = queue.drain(..).collect();
        sorted_queue.sort_by_key(|idx| {
            self.graph
                .node_weight(*idx)
                .map(|n| n.id.raw())
                .unwrap_or(0)
        });
        queue.extend(sorted_queue);

        let mut result = Vec::new();

        while let Some(idx) = queue.pop_front() {
            if let Some(node) = self.graph.node_weight(idx) {
                result.push(node.id);
            }
            let mut neighbors: Vec<NodeIndex> = self
                .graph
                .neighbors_directed(idx, Direction::Outgoing)
                .collect();
            neighbors.sort_by_key(|n| {
                self.graph
                    .node_weight(*n)
                    .map(|node| node.id.raw())
                    .unwrap_or(0)
            });
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(&neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        result
    }

    /// Returns only the dirty nodes, in topological order.
    pub fn dirty_nodes(&self) -> Vec<NodeId> {
        self.topological_order()
            .into_iter()
            .filter(|id| {
                self.id_to_index
                    .get(id)
                    .and_then(|idx| self.graph.node_weight(*idx))
                    .map_or(false, |n| n.dirty)
            })
            .collect()
    }

    pub fn node(&self, id: NodeId) -> Option<&NodeInstance> {
        self.id_to_index
            .get(&id)
            .and_then(|idx| self.graph.node_weight(*idx))
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut NodeInstance> {
        self.id_to_index
            .get(&id)
            .copied()
            .and_then(move |idx| self.graph.node_weight_mut(idx))
    }

    /// Returns outgoing connections from a node as
    /// `(from_node, from_port, to_node, to_port)`.
    pub fn connections_from(&self, id: NodeId) -> Vec<(NodeId, usize, NodeId, usize)> {
        let idx = match self.id_to_index.get(&id) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| {
                let from_id = self.graph.node_weight(e.source()).unwrap().id;
                let to_id = self.graph.node_weight(e.target()).unwrap().id;
                (from_id, e.weight().from_port, to_id, e.weight().to_port)
            })
            .collect()
    }

    /// Returns incoming connections to a node as
    /// `(from_node, from_port, to_node, to_port)`.
    pub fn connections_to(&self, id: NodeId) -> Vec<(NodeId, usize, NodeId, usize)> {
        let idx = match self.id_to_index.get(&id) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .map(|e| {
                let from_id = self.graph.node_weight(e.source()).unwrap().id;
                let to_id = self.graph.node_weight(e.target()).unwrap().id;
                (from_id, e.weight().from_port, to_id, e.weight().to_port)
            })
            .collect()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn connection_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = &NodeInstance> {
        self.graph.node_weights()
    }

    pub fn set_input(&mut self, id: NodeId, port: usize, value: crate::node::NodeValue) {
        if let Some(node) = self.node_mut(id) {
            while node.input_values.len() <= port {
                node.input_values.push(crate::node::NodeValue::None);
            }
            node.input_values[port] = value;
        }
    }

    pub fn clear(&mut self) {
        self.graph.clear();
        self.id_to_index.clear();
    }

    // --- private helpers ---

    /// DFS reachability check: can `from` reach `to` following directed edges?
    fn can_reach(&self, from: NodeIndex, to: NodeIndex) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![from];
        while let Some(current) = stack.pop() {
            if current == to {
                return true;
            }
            if visited.insert(current) {
                for neighbor in self.graph.neighbors_directed(current, Direction::Outgoing) {
                    stack.push(neighbor);
                }
            }
        }
        false
    }
}

impl Default for NodeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_count_nodes() {
        let mut g = NodeGraph::new();
        assert_eq!(g.node_count(), 0);
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        assert_eq!(g.node_count(), 2);
        assert!(g.node(a).is_some());
        assert!(g.node(b).is_some());
    }

    #[test]
    fn remove_node() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        assert!(g.remove_node(a));
        assert_eq!(g.node_count(), 0);
        assert!(!g.remove_node(a)); // already removed
    }

    #[test]
    fn connect_and_disconnect() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        g.connect(a, 0, b, 0).unwrap();
        assert_eq!(g.connection_count(), 1);

        let conns = g.connections_from(a);
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0], (a, 0, b, 0));

        let incoming = g.connections_to(b);
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0], (a, 0, b, 0));

        assert!(g.disconnect(a, 0, b, 0));
        assert_eq!(g.connection_count(), 0);
        assert!(!g.disconnect(a, 0, b, 0)); // already removed
    }

    #[test]
    fn cycle_detection_direct() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        g.connect(a, 0, b, 0).unwrap();
        // b -> a would create a cycle
        let result = g.connect(b, 0, a, 0);
        assert!(result.is_err());
    }

    #[test]
    fn cycle_detection_indirect() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        let c = g.add_node("C".into(), (2.0, 0.0));
        g.connect(a, 0, b, 0).unwrap();
        g.connect(b, 0, c, 0).unwrap();
        // c -> a would close the cycle
        let result = g.connect(c, 0, a, 0);
        assert!(result.is_err());
    }

    #[test]
    fn self_loop_rejected() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let result = g.connect(a, 0, a, 0);
        assert!(result.is_err());
    }

    #[test]
    fn dirty_propagation() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        let c = g.add_node("C".into(), (2.0, 0.0));
        let d = g.add_node("D".into(), (3.0, 0.0)); // unconnected

        g.connect(a, 0, b, 0).unwrap();
        g.connect(b, 0, c, 0).unwrap();

        // Clear all dirty flags manually.
        g.node_mut(a).unwrap().dirty = false;
        g.node_mut(b).unwrap().dirty = false;
        g.node_mut(c).unwrap().dirty = false;
        g.node_mut(d).unwrap().dirty = false;

        // Mark A dirty -> B and C should become dirty too, D should not.
        g.mark_dirty(a);
        assert!(g.node(a).unwrap().dirty);
        assert!(g.node(b).unwrap().dirty);
        assert!(g.node(c).unwrap().dirty);
        assert!(!g.node(d).unwrap().dirty);
    }

    #[test]
    fn topological_order_respects_edges() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        let c = g.add_node("C".into(), (2.0, 0.0));
        g.connect(a, 0, b, 0).unwrap();
        g.connect(b, 0, c, 0).unwrap();

        let order = g.topological_order();
        let pos_a = order.iter().position(|&id| id == a).unwrap();
        let pos_b = order.iter().position(|&id| id == b).unwrap();
        let pos_c = order.iter().position(|&id| id == c).unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn dirty_nodes_in_topological_order() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        let c = g.add_node("C".into(), (2.0, 0.0));
        g.connect(a, 0, b, 0).unwrap();
        g.connect(b, 0, c, 0).unwrap();

        // Clear dirty on A only.
        g.node_mut(a).unwrap().dirty = false;

        let dirty = g.dirty_nodes();
        // b and c should be dirty (from connect), a should not be.
        assert!(!dirty.contains(&a));
        assert!(dirty.contains(&b));
        assert!(dirty.contains(&c));

        // Verify order: b before c.
        let pos_b = dirty.iter().position(|&id| id == b).unwrap();
        let pos_c = dirty.iter().position(|&id| id == c).unwrap();
        assert!(pos_b < pos_c);
    }

    #[test]
    fn remove_node_removes_connections() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        let c = g.add_node("C".into(), (2.0, 0.0));
        g.connect(a, 0, b, 0).unwrap();
        g.connect(b, 0, c, 0).unwrap();
        assert_eq!(g.connection_count(), 2);

        g.remove_node(b);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.connection_count(), 0);
    }

    #[test]
    fn clear_empties_graph() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (1.0, 0.0));
        g.connect(a, 0, b, 0).unwrap();
        g.clear();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.connection_count(), 0);
    }

    #[test]
    fn diamond_topology() {
        //   A
        //  / \
        // B   C
        //  \ /
        //   D
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let b = g.add_node("B".into(), (0.0, 1.0));
        let c = g.add_node("C".into(), (1.0, 1.0));
        let d = g.add_node("D".into(), (0.5, 2.0));
        g.connect(a, 0, b, 0).unwrap();
        g.connect(a, 1, c, 0).unwrap();
        g.connect(b, 0, d, 0).unwrap();
        g.connect(c, 0, d, 1).unwrap();

        let order = g.topological_order();
        let pos = |id: NodeId| order.iter().position(|&x| x == id).unwrap();
        assert!(pos(a) < pos(b));
        assert!(pos(a) < pos(c));
        assert!(pos(b) < pos(d));
        assert!(pos(c) < pos(d));
    }

    #[test]
    fn connect_nonexistent_node_errors() {
        let mut g = NodeGraph::new();
        let a = g.add_node("A".into(), (0.0, 0.0));
        let fake = NodeId::new();
        assert!(g.connect(a, 0, fake, 0).is_err());
        assert!(g.connect(fake, 0, a, 0).is_err());
    }
}
