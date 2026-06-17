use std::collections::HashMap;

use toolkit_core::{ToolkitError, ToolkitResult};

use crate::dag::NodeGraph;
use crate::node::{DataType, NodeTemplate, NodeValue, PortDefinition};

/// Registry of node templates keyed by name.
pub struct NodeRegistry {
    templates: HashMap<String, Box<dyn NodeTemplate>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn register(&mut self, template: impl NodeTemplate + 'static) {
        self.templates
            .insert(template.name().to_owned(), Box::new(template));
    }

    pub fn get(&self, name: &str) -> Option<&dyn NodeTemplate> {
        self.templates.get(name).map(|b| b.as_ref())
    }

    pub fn template_names(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluate all dirty nodes in a graph.
///
/// For each dirty node (in topological order):
/// 1. Gather inputs: if a port has an incoming connection, read the cached
///    output of the source node at the source port; otherwise use the node's
///    own `input_values` entry or `NodeValue::None`.
/// 2. Look up the template in `registry` and call `evaluate`.
/// 3. Store results in `cached_outputs` and clear the dirty flag.
pub fn evaluate_graph(graph: &mut NodeGraph, registry: &NodeRegistry) -> ToolkitResult<()> {
    let dirty = graph.dirty_nodes();

    for node_id in dirty {
        // Determine template name and how many inputs we need.
        let template_name = {
            let node = graph
                .node(node_id)
                .ok_or_else(|| ToolkitError::Custom("Node disappeared during evaluation".into()))?;
            node.template_name.clone()
        };

        let template = registry.get(&template_name).ok_or_else(|| {
            ToolkitError::Custom(format!("No template registered for '{template_name}'"))
        })?;

        let input_defs = template.inputs();
        let incoming = graph.connections_to(node_id);

        // Build the input vector.
        let mut inputs: Vec<NodeValue> = Vec::with_capacity(input_defs.len());
        for (port_idx, def) in input_defs.iter().enumerate() {
            // Check if there's an incoming connection to this port.
            let connected_value = incoming.iter().find_map(|&(src_id, src_port, _, dst_port)| {
                if dst_port == port_idx {
                    // Read the cached output of the source node.
                    graph.node(src_id).and_then(|src_node| {
                        src_node
                            .cached_outputs
                            .as_ref()
                            .and_then(|outputs| outputs.get(src_port).cloned())
                    })
                } else {
                    None
                }
            });

            if let Some(val) = connected_value {
                inputs.push(val);
            } else {
                // Use the node instance's own input value, or the port default, or None.
                let node = graph.node(node_id).unwrap();
                let val = node
                    .input_values
                    .get(port_idx)
                    .cloned()
                    .or_else(|| def.default_value.clone())
                    .unwrap_or(NodeValue::None);
                inputs.push(val);
            }
        }

        // Evaluate.
        let outputs = template.evaluate(&inputs);

        // Store results.
        let node = graph.node_mut(node_id).unwrap();
        node.cached_outputs = Some(outputs);
        node.dirty = false;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Built-in templates
// ---------------------------------------------------------------------------

/// Outputs a constant float.
pub struct FloatConstant;

impl NodeTemplate for FloatConstant {
    fn name(&self) -> &str {
        "FloatConstant"
    }

    fn inputs(&self) -> Vec<PortDefinition> {
        vec![PortDefinition::new("value", DataType::Float).with_default(NodeValue::Float(0.0))]
    }

    fn outputs(&self) -> Vec<PortDefinition> {
        vec![PortDefinition::new("out", DataType::Float)]
    }

    fn evaluate(&self, inputs: &[NodeValue]) -> Vec<NodeValue> {
        let v = inputs.first().map(|i| i.as_float()).unwrap_or(0.0);
        vec![NodeValue::Float(v)]
    }
}

/// Adds two floats.
pub struct AddFloat;

impl NodeTemplate for AddFloat {
    fn name(&self) -> &str {
        "AddFloat"
    }

    fn inputs(&self) -> Vec<PortDefinition> {
        vec![
            PortDefinition::new("a", DataType::Float).with_default(NodeValue::Float(0.0)),
            PortDefinition::new("b", DataType::Float).with_default(NodeValue::Float(0.0)),
        ]
    }

    fn outputs(&self) -> Vec<PortDefinition> {
        vec![PortDefinition::new("sum", DataType::Float)]
    }

    fn evaluate(&self, inputs: &[NodeValue]) -> Vec<NodeValue> {
        let a = inputs.first().map(|i| i.as_float()).unwrap_or(0.0);
        let b = inputs.get(1).map(|i| i.as_float()).unwrap_or(0.0);
        vec![NodeValue::Float(a + b)]
    }
}

/// Multiplies two floats.
pub struct MultiplyFloat;

impl NodeTemplate for MultiplyFloat {
    fn name(&self) -> &str {
        "MultiplyFloat"
    }

    fn inputs(&self) -> Vec<PortDefinition> {
        vec![
            PortDefinition::new("a", DataType::Float).with_default(NodeValue::Float(0.0)),
            PortDefinition::new("b", DataType::Float).with_default(NodeValue::Float(1.0)),
        ]
    }

    fn outputs(&self) -> Vec<PortDefinition> {
        vec![PortDefinition::new("product", DataType::Float)]
    }

    fn evaluate(&self, inputs: &[NodeValue]) -> Vec<NodeValue> {
        let a = inputs.first().map(|i| i.as_float()).unwrap_or(0.0);
        let b = inputs.get(1).map(|i| i.as_float()).unwrap_or(1.0);
        vec![NodeValue::Float(a * b)]
    }
}

/// Linear interpolation between two floats: `mix = a*(1-t) + b*t`.
pub struct MixFloat;

impl NodeTemplate for MixFloat {
    fn name(&self) -> &str {
        "MixFloat"
    }

    fn inputs(&self) -> Vec<PortDefinition> {
        vec![
            PortDefinition::new("a", DataType::Float).with_default(NodeValue::Float(0.0)),
            PortDefinition::new("b", DataType::Float).with_default(NodeValue::Float(1.0)),
            PortDefinition::new("factor", DataType::Float).with_default(NodeValue::Float(0.5)),
        ]
    }

    fn outputs(&self) -> Vec<PortDefinition> {
        vec![PortDefinition::new("result", DataType::Float)]
    }

    fn evaluate(&self, inputs: &[NodeValue]) -> Vec<NodeValue> {
        let a = inputs.first().map(|i| i.as_float()).unwrap_or(0.0);
        let b = inputs.get(1).map(|i| i.as_float()).unwrap_or(1.0);
        let t = inputs.get(2).map(|i| i.as_float()).unwrap_or(0.5);
        let t = t.clamp(0.0, 1.0);
        vec![NodeValue::Float(a * (1.0 - t) + b * t)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registry() -> NodeRegistry {
        let mut reg = NodeRegistry::new();
        reg.register(FloatConstant);
        reg.register(AddFloat);
        reg.register(MultiplyFloat);
        reg.register(MixFloat);
        reg
    }

    #[test]
    fn evaluate_simple_chain() {
        // Constant(5.0) -> port 0 of Add
        // Constant(3.0) -> port 1 of Add
        // Expected result: 8.0
        let registry = make_registry();
        let mut graph = NodeGraph::new();

        let c1 = graph.add_node("FloatConstant".into(), (0.0, 0.0));
        let c2 = graph.add_node("FloatConstant".into(), (0.0, 1.0));
        let add = graph.add_node("AddFloat".into(), (1.0, 0.5));

        // Set constant values via input_values.
        graph.node_mut(c1).unwrap().input_values = vec![NodeValue::Float(5.0)];
        graph.node_mut(c2).unwrap().input_values = vec![NodeValue::Float(3.0)];

        graph.connect(c1, 0, add, 0).unwrap();
        graph.connect(c2, 0, add, 1).unwrap();

        evaluate_graph(&mut graph, &registry).unwrap();

        let result = graph.node(add).unwrap().cached_outputs.as_ref().unwrap();
        assert_eq!(result[0], NodeValue::Float(8.0));
    }

    #[test]
    fn evaluate_chain_constant_add_multiply() {
        // C1(4.0) --\
        //            Add --> Mul --> result
        // C2(6.0) --/        ^
        //                    |
        // C3(2.0) ----------/
        // Expected: (4+6) * 2 = 20
        let registry = make_registry();
        let mut graph = NodeGraph::new();

        let c1 = graph.add_node("FloatConstant".into(), (0.0, 0.0));
        let c2 = graph.add_node("FloatConstant".into(), (0.0, 1.0));
        let c3 = graph.add_node("FloatConstant".into(), (0.0, 2.0));
        let add = graph.add_node("AddFloat".into(), (1.0, 0.5));
        let mul = graph.add_node("MultiplyFloat".into(), (2.0, 0.5));

        graph.node_mut(c1).unwrap().input_values = vec![NodeValue::Float(4.0)];
        graph.node_mut(c2).unwrap().input_values = vec![NodeValue::Float(6.0)];
        graph.node_mut(c3).unwrap().input_values = vec![NodeValue::Float(2.0)];

        graph.connect(c1, 0, add, 0).unwrap();
        graph.connect(c2, 0, add, 1).unwrap();
        graph.connect(add, 0, mul, 0).unwrap();
        graph.connect(c3, 0, mul, 1).unwrap();

        evaluate_graph(&mut graph, &registry).unwrap();

        let result = graph.node(mul).unwrap().cached_outputs.as_ref().unwrap();
        assert_eq!(result[0], NodeValue::Float(20.0));
    }

    #[test]
    fn re_evaluate_only_dirty() {
        let registry = make_registry();
        let mut graph = NodeGraph::new();

        let c1 = graph.add_node("FloatConstant".into(), (0.0, 0.0));
        let c2 = graph.add_node("FloatConstant".into(), (0.0, 1.0));
        let add = graph.add_node("AddFloat".into(), (1.0, 0.5));

        graph.node_mut(c1).unwrap().input_values = vec![NodeValue::Float(5.0)];
        graph.node_mut(c2).unwrap().input_values = vec![NodeValue::Float(3.0)];

        graph.connect(c1, 0, add, 0).unwrap();
        graph.connect(c2, 0, add, 1).unwrap();

        // First evaluation.
        evaluate_graph(&mut graph, &registry).unwrap();
        assert_eq!(
            graph.node(add).unwrap().cached_outputs.as_ref().unwrap()[0],
            NodeValue::Float(8.0)
        );
        // All should be clean now.
        assert!(graph.dirty_nodes().is_empty());

        // Now change c1 to 10.
        graph.node_mut(c1).unwrap().input_values = vec![NodeValue::Float(10.0)];
        graph.mark_dirty(c1);

        // c2 should NOT be dirty.
        assert!(!graph.node(c2).unwrap().dirty);
        // c1 and add should be dirty.
        assert!(graph.node(c1).unwrap().dirty);
        assert!(graph.node(add).unwrap().dirty);

        evaluate_graph(&mut graph, &registry).unwrap();
        assert_eq!(
            graph.node(add).unwrap().cached_outputs.as_ref().unwrap()[0],
            NodeValue::Float(13.0)
        );
    }

    #[test]
    fn unconnected_inputs_use_defaults() {
        let registry = make_registry();
        let mut graph = NodeGraph::new();

        // MixFloat with no connections -- should use defaults: a=0, b=1, t=0.5
        // result = 0*(1-0.5) + 1*0.5 = 0.5
        let mix = graph.add_node("MixFloat".into(), (0.0, 0.0));
        evaluate_graph(&mut graph, &registry).unwrap();

        let result = graph.node(mix).unwrap().cached_outputs.as_ref().unwrap();
        assert!((result[0].as_float() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mix_float_lerp() {
        let registry = make_registry();
        let mut graph = NodeGraph::new();

        let ca = graph.add_node("FloatConstant".into(), (0.0, 0.0));
        let cb = graph.add_node("FloatConstant".into(), (0.0, 1.0));
        let ct = graph.add_node("FloatConstant".into(), (0.0, 2.0));
        let mix = graph.add_node("MixFloat".into(), (1.0, 1.0));

        graph.node_mut(ca).unwrap().input_values = vec![NodeValue::Float(10.0)];
        graph.node_mut(cb).unwrap().input_values = vec![NodeValue::Float(20.0)];
        graph.node_mut(ct).unwrap().input_values = vec![NodeValue::Float(0.25)];

        graph.connect(ca, 0, mix, 0).unwrap();
        graph.connect(cb, 0, mix, 1).unwrap();
        graph.connect(ct, 0, mix, 2).unwrap();

        evaluate_graph(&mut graph, &registry).unwrap();

        // 10*(1-0.25) + 20*0.25 = 7.5 + 5.0 = 12.5
        let result = graph.node(mix).unwrap().cached_outputs.as_ref().unwrap();
        assert!((result[0].as_float() - 12.5).abs() < 1e-6);
    }

    #[test]
    fn registry_unknown_template_errors() {
        let registry = NodeRegistry::new(); // empty
        let mut graph = NodeGraph::new();
        graph.add_node("NonExistent".into(), (0.0, 0.0));
        let result = evaluate_graph(&mut graph, &registry);
        assert!(result.is_err());
    }
}
