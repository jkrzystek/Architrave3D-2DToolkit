use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{json, Value};
use toolkit_graph::{NodeGraph, NodeRegistry, NodeValue};

use crate::error::{BridgeError, BridgeResult};
use crate::provider::*;

/// AI bridge adapter for the procedural node graph.
///
/// Exposes graph topology, node properties, and evaluation — NOT raw
/// intermediate buffers or image data flowing between nodes.
pub struct GraphBridge {
    graph: Arc<RwLock<NodeGraph>>,
    registry: Arc<RwLock<NodeRegistry>>,
}

impl GraphBridge {
    pub fn new(graph: Arc<RwLock<NodeGraph>>, registry: Arc<RwLock<NodeRegistry>>) -> Self {
        Self { graph, registry }
    }
}

impl AiProvider for GraphBridge {
    fn namespace(&self) -> &str {
        "graph"
    }

    fn description(&self) -> &str {
        "Procedural node graph management. View/edit nodes, connections, and \
         trigger evaluation. Exposes topology and values, not intermediate buffers."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![
            ResourceDescriptor::json(
                "graph://overview",
                "Graph Overview",
                "Node count, connection count, dirty node count",
            ),
            ResourceDescriptor::json(
                "graph://nodes",
                "Node List",
                "All nodes with template name, position, and dirty state",
            ),
            ResourceDescriptor::json(
                "graph://templates",
                "Available Templates",
                "All registered node templates with input/output port definitions",
            ),
        ]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "graph.add_node",
                "Add a node to the graph",
                json!({
                    "type": "object",
                    "properties": {
                        "template": {"type": "string", "description": "Template name (e.g. FloatConstant, AddFloat)"},
                        "x": {"type": "number", "default": 0.0, "description": "X position in graph editor"},
                        "y": {"type": "number", "default": 0.0, "description": "Y position in graph editor"},
                    },
                    "required": ["template"]
                }),
            ),
            ToolDescriptor::new(
                "graph.remove_node",
                "Remove a node from the graph",
                json!({
                    "type": "object",
                    "properties": {
                        "node_id": {"type": "string"},
                    },
                    "required": ["node_id"]
                }),
            ),
            ToolDescriptor::new(
                "graph.connect",
                "Connect two nodes (output port to input port)",
                json!({
                    "type": "object",
                    "properties": {
                        "from_node": {"type": "string"},
                        "from_port": {"type": "integer", "minimum": 0},
                        "to_node": {"type": "string"},
                        "to_port": {"type": "integer", "minimum": 0},
                    },
                    "required": ["from_node", "from_port", "to_node", "to_port"]
                }),
            ),
            ToolDescriptor::new(
                "graph.disconnect",
                "Disconnect two nodes",
                json!({
                    "type": "object",
                    "properties": {
                        "from_node": {"type": "string"},
                        "from_port": {"type": "integer"},
                        "to_node": {"type": "string"},
                        "to_port": {"type": "integer"},
                    },
                    "required": ["from_node", "from_port", "to_node", "to_port"]
                }),
            ),
            ToolDescriptor::new(
                "graph.set_input_value",
                "Set a node's input port value directly",
                json!({
                    "type": "object",
                    "properties": {
                        "node_id": {"type": "string"},
                        "port": {"type": "integer"},
                        "value": {"type": "number", "description": "Float value to set"},
                    },
                    "required": ["node_id", "port", "value"]
                }),
            ),
            ToolDescriptor::new(
                "graph.evaluate",
                "Evaluate all dirty nodes in topological order",
                json!({"type": "object", "properties": {}}),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        let graph = self.graph.read();
        match uri {
            "graph://overview" => {
                let node_count = graph.node_count();
                let dirty_count = graph.dirty_nodes().len();
                ResourceContent::json(
                    uri,
                    &json!({
                        "node_count": node_count,
                        "dirty_node_count": dirty_count,
                    }),
                )
            }
            "graph://nodes" => {
                let nodes: Vec<Value> = graph
                    .all_nodes()
                    .map(|n| {
                        json!({
                            "id": n.id.to_string(),
                            "template": n.template_name,
                            "position": [n.position.0, n.position.1],
                            "dirty": n.dirty,
                        })
                    })
                    .collect();
                ResourceContent::json(uri, &json!({"nodes": nodes}))
            }
            "graph://templates" => {
                let reg = self.registry.read();
                let names = reg.template_names();
                let templates: Vec<Value> = names
                    .iter()
                    .filter_map(|name| {
                        reg.get(name).map(|t| {
                            let inputs: Vec<Value> = t
                                .inputs()
                                .iter()
                                .map(|p| json!({"name": p.name, "type": format!("{:?}", p.data_type)}))
                                .collect();
                            let outputs: Vec<Value> = t
                                .outputs()
                                .iter()
                                .map(|p| json!({"name": p.name, "type": format!("{:?}", p.data_type)}))
                                .collect();
                            json!({
                                "name": t.name(),
                                "inputs": inputs,
                                "outputs": outputs,
                            })
                        })
                    })
                    .collect();
                ResourceContent::json(uri, &json!({"templates": templates}))
            }
            _ => Err(BridgeError::ResourceNotFound(uri.into())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        let mut graph = self.graph.write();
        match name {
            "graph.add_node" => {
                let template = args
                    .get("template")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'template'".into()))?;
                let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let id = graph.add_node(template.into(), (x, y));
                ToolResult::success_json(&json!({
                    "node_id": id.to_string(),
                    "template": template,
                }))
            }
            "graph.remove_node" => {
                let id = parse_node_id(&args)?;
                if graph.remove_node(id) {
                    ToolResult::success_json(&json!({"removed": id.to_string()}))
                } else {
                    Ok(ToolResult::error("Node not found"))
                }
            }
            "graph.connect" => {
                let from = parse_node_id_field(&args, "from_node")?;
                let from_port = args.get("from_port").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let to = parse_node_id_field(&args, "to_node")?;
                let to_port = args.get("to_port").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                graph
                    .connect(from, from_port, to, to_port)
                    .map_err(|e| BridgeError::InvalidArguments(e.to_string()))?;
                ToolResult::success_json(&json!({"connected": true}))
            }
            "graph.disconnect" => {
                let from = parse_node_id_field(&args, "from_node")?;
                let from_port = args.get("from_port").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let to = parse_node_id_field(&args, "to_node")?;
                let to_port = args.get("to_port").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                graph.disconnect(from, from_port, to, to_port);
                ToolResult::success_json(&json!({"disconnected": true}))
            }
            "graph.set_input_value" => {
                let id = parse_node_id(&args)?;
                let port = args.get("port").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let value = args
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'value'".into()))?
                    as f32;
                graph.set_input(id, port, NodeValue::Float(value));
                graph.mark_dirty(id);
                ToolResult::success_json(&json!({
                    "node_id": id.to_string(),
                    "port": port,
                    "value": value,
                }))
            }
            "graph.evaluate" => {
                let reg = self.registry.read();
                let dirty_before = graph.dirty_nodes().len();
                let _ = toolkit_graph::evaluate_graph(&mut graph, &reg);
                ToolResult::success_json(&json!({
                    "evaluated": true,
                    "dirty_nodes_before": dirty_before,
                    "dirty_nodes_after": graph.dirty_nodes().len(),
                }))
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

fn parse_node_id(args: &Value) -> BridgeResult<toolkit_core::NodeId> {
    parse_node_id_field(args, "node_id")
}

fn parse_node_id_field(args: &Value, field: &str) -> BridgeResult<toolkit_core::NodeId> {
    let s = args
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| BridgeError::InvalidArguments(format!("missing '{field}'")))?;
    let n: u64 = s
        .parse()
        .map_err(|_| BridgeError::InvalidArguments(format!("invalid node id: {s}")))?;
    Ok(toolkit_core::NodeId::from_raw(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bridge() -> GraphBridge {
        let graph = Arc::new(RwLock::new(NodeGraph::new()));
        let mut registry = NodeRegistry::new();
        registry.register(toolkit_graph::FloatConstant);
        registry.register(toolkit_graph::AddFloat);
        let registry = Arc::new(RwLock::new(registry));
        GraphBridge::new(graph, registry)
    }

    #[test]
    fn add_node_and_list() {
        let bridge = make_bridge();
        bridge
            .call_tool("graph.add_node", json!({"template": "FloatConstant", "x": 10.0}))
            .unwrap();

        let content = bridge.read_resource("graph://overview").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["node_count"], 1);
    }

    #[test]
    fn list_templates() {
        let bridge = make_bridge();
        let content = bridge.read_resource("graph://templates").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        let templates = v["templates"].as_array().unwrap();
        assert!(templates.len() >= 2);
    }

    #[test]
    fn connect_and_evaluate() {
        let bridge = make_bridge();
        let r1 = bridge
            .call_tool("graph.add_node", json!({"template": "FloatConstant"}))
            .unwrap();
        let r2 = bridge
            .call_tool("graph.add_node", json!({"template": "FloatConstant"}))
            .unwrap();
        let r3 = bridge
            .call_tool("graph.add_node", json!({"template": "AddFloat"}))
            .unwrap();

        let id1 = extract_node_id(&r1);
        let id2 = extract_node_id(&r2);
        let id3 = extract_node_id(&r3);

        bridge
            .call_tool(
                "graph.connect",
                json!({"from_node": id1, "from_port": 0, "to_node": id3, "to_port": 0}),
            )
            .unwrap();
        bridge
            .call_tool(
                "graph.connect",
                json!({"from_node": id2, "from_port": 0, "to_node": id3, "to_port": 1}),
            )
            .unwrap();

        let result = bridge.call_tool("graph.evaluate", json!({})).unwrap();
        assert!(!result.is_error);
    }

    fn extract_node_id(result: &ToolResult) -> String {
        let text = match &result.content[0] {
            ToolResultContent::Text { text } => text,
        };
        let v: Value = serde_json::from_str(text).unwrap();
        v["node_id"].as_str().unwrap().to_string()
    }
}
