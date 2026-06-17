use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{json, Value};
use toolkit_scene::{NodeData, NodeKey, Scene, Transform};

use crate::error::{BridgeError, BridgeResult};
use crate::provider::*;

/// AI bridge adapter for the scene graph.
///
/// Exposes the node hierarchy and transform editing through semantic
/// operations. Nodes are addressed by a stable `"index:generation"` id. Only
/// structure and transforms are exposed — never vertex data or GPU handles.
pub struct SceneBridge {
    scene: Arc<RwLock<Scene>>,
}

impl SceneBridge {
    pub fn new(scene: Arc<RwLock<Scene>>) -> Self {
        Self { scene }
    }
}

fn key_to_id(key: NodeKey) -> String {
    format!("{}:{}", key.index(), key.generation())
}

fn parse_id(id: &str) -> BridgeResult<NodeKey> {
    let (i, g) = id
        .split_once(':')
        .ok_or_else(|| BridgeError::InvalidArguments(format!("bad node id '{id}'")))?;
    let index = i
        .parse::<u32>()
        .map_err(|_| BridgeError::InvalidArguments(format!("bad node id '{id}'")))?;
    let generation = g
        .parse::<u32>()
        .map_err(|_| BridgeError::InvalidArguments(format!("bad node id '{id}'")))?;
    Ok(NodeKey::from_raw_parts(index, generation))
}

fn data_kind(data: &NodeData) -> &'static str {
    match data {
        NodeData::Empty => "empty",
        NodeData::Mesh { .. } => "mesh",
        NodeData::Light(_) => "light",
        NodeData::Camera => "camera",
    }
}

fn require_id(args: &Value) -> BridgeResult<NodeKey> {
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BridgeError::InvalidArguments("missing 'id'".into()))?;
    parse_id(id)
}

impl AiProvider for SceneBridge {
    fn namespace(&self) -> &str {
        "scene"
    }

    fn description(&self) -> &str {
        "3D scene graph: inspect the node hierarchy and edit transforms, \
         visibility, and parenting. Geometry is referenced by id, not exposed raw."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![
            ResourceDescriptor::json(
                "scene://overview",
                "Scene Overview",
                "Node and root counts for the scene",
            ),
            ResourceDescriptor::json(
                "scene://nodes",
                "Scene Nodes",
                "All nodes with id, name, kind, visibility, parent, and local translation",
            ),
        ]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "scene.add_node",
                "Add an empty transform node, optionally under a parent",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "parent": {"type": "string", "description": "Parent node id, or omit for root"},
                        "translation": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}
                    },
                    "required": ["name"]
                }),
            ),
            ToolDescriptor::new(
                "scene.remove_node",
                "Remove a node and its entire subtree",
                json!({
                    "type": "object",
                    "properties": {"id": {"type": "string"}},
                    "required": ["id"]
                }),
            ),
            ToolDescriptor::new(
                "scene.set_translation",
                "Set a node's local translation",
                json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "x": {"type": "number"}, "y": {"type": "number"}, "z": {"type": "number"}
                    },
                    "required": ["id", "x", "y", "z"]
                }),
            ),
            ToolDescriptor::new(
                "scene.set_scale",
                "Set a node's local scale",
                json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "x": {"type": "number"}, "y": {"type": "number"}, "z": {"type": "number"}
                    },
                    "required": ["id", "x", "y", "z"]
                }),
            ),
            ToolDescriptor::new(
                "scene.set_visible",
                "Show or hide a node (and its subtree)",
                json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "visible": {"type": "boolean"}
                    },
                    "required": ["id", "visible"]
                }),
            ),
            ToolDescriptor::new(
                "scene.reparent",
                "Move a node under a new parent (or to the root if parent omitted)",
                json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "parent": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        match uri {
            "scene://overview" => {
                let scene = self.scene.read();
                ResourceContent::json(
                    uri,
                    &json!({
                        "node_count": scene.len(),
                        "root_count": scene.roots().len(),
                    }),
                )
            }
            "scene://nodes" => {
                let scene = self.scene.read();
                let nodes: Vec<Value> = scene
                    .iter()
                    .map(|(key, node)| {
                        let t = node.transform.translation;
                        json!({
                            "id": key_to_id(key),
                            "name": node.name,
                            "kind": data_kind(&node.data),
                            "visible": node.visible,
                            "parent": node.parent().map(key_to_id),
                            "translation": [t.x, t.y, t.z],
                        })
                    })
                    .collect();
                ResourceContent::json(uri, &json!({ "nodes": nodes }))
            }
            _ => Err(BridgeError::ResourceNotFound(uri.into())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        match name {
            "scene.add_node" => {
                let node_name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("node")
                    .to_string();
                let mut transform = Transform::IDENTITY;
                if let Some(t) = args.get("translation").and_then(|v| v.as_array()) {
                    if t.len() == 3 {
                        transform.translation = glam::Vec3::new(
                            t[0].as_f64().unwrap_or(0.0) as f32,
                            t[1].as_f64().unwrap_or(0.0) as f32,
                            t[2].as_f64().unwrap_or(0.0) as f32,
                        );
                    }
                }
                let mut scene = self.scene.write();
                let key = match args.get("parent").and_then(|v| v.as_str()) {
                    Some(pid) => {
                        let parent = parse_id(pid)?;
                        scene
                            .add_child(parent, node_name, transform, NodeData::Empty)
                            .ok_or_else(|| {
                                BridgeError::InvalidArguments("invalid parent id".into())
                            })?
                    }
                    None => scene.add_node(node_name, transform, NodeData::Empty),
                };
                ToolResult::success_json(&json!({ "id": key_to_id(key) }))
            }
            "scene.remove_node" => {
                let key = require_id(&args)?;
                let removed = self.scene.write().remove(key);
                if removed == 0 {
                    return Err(BridgeError::InvalidArguments("invalid node id".into()));
                }
                ToolResult::success_json(&json!({ "removed": removed }))
            }
            "scene.set_translation" => {
                let key = require_id(&args)?;
                let (x, y, z) = xyz(&args)?;
                let mut scene = self.scene.write();
                let node = scene
                    .get_mut(key)
                    .ok_or_else(|| BridgeError::InvalidArguments("invalid node id".into()))?;
                node.transform.translation = glam::Vec3::new(x, y, z);
                ToolResult::success_json(&json!({ "translation": [x, y, z] }))
            }
            "scene.set_scale" => {
                let key = require_id(&args)?;
                let (x, y, z) = xyz(&args)?;
                let mut scene = self.scene.write();
                let node = scene
                    .get_mut(key)
                    .ok_or_else(|| BridgeError::InvalidArguments("invalid node id".into()))?;
                node.transform.scale = glam::Vec3::new(x, y, z);
                ToolResult::success_json(&json!({ "scale": [x, y, z] }))
            }
            "scene.set_visible" => {
                let key = require_id(&args)?;
                let visible = args
                    .get("visible")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'visible'".into()))?;
                let mut scene = self.scene.write();
                let node = scene
                    .get_mut(key)
                    .ok_or_else(|| BridgeError::InvalidArguments("invalid node id".into()))?;
                node.visible = visible;
                ToolResult::success_json(&json!({ "visible": visible }))
            }
            "scene.reparent" => {
                let key = require_id(&args)?;
                let parent = match args.get("parent").and_then(|v| v.as_str()) {
                    Some(pid) => Some(parse_id(pid)?),
                    None => None,
                };
                let ok = self.scene.write().set_parent(key, parent);
                if !ok {
                    return Err(BridgeError::InvalidArguments(
                        "reparent failed (invalid id or would create a cycle)".into(),
                    ));
                }
                ToolResult::success_json(&json!({ "reparented": true }))
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

fn xyz(args: &Value) -> BridgeResult<(f32, f32, f32)> {
    let g = |k: &str| {
        args.get(k)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| BridgeError::InvalidArguments(format!("missing '{k}'")))
            .map(|v| v as f32)
    };
    Ok((g("x")?, g("y")?, g("z")?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bridge() -> SceneBridge {
        SceneBridge::new(Arc::new(RwLock::new(Scene::new())))
    }

    fn result_text(res: &ToolResult) -> String {
        match &res.content[0] {
            ToolResultContent::Text { text } => text.clone(),
        }
    }

    #[test]
    fn add_then_list_node() {
        let bridge = make_bridge();
        let res = bridge
            .call_tool("scene.add_node", json!({"name": "root"}))
            .unwrap();
        assert!(!res.is_error);

        let content = bridge.read_resource("scene://nodes").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(v["nodes"][0]["name"], "root");
    }

    #[test]
    fn set_translation_updates_node() {
        let bridge = make_bridge();
        let res = bridge
            .call_tool("scene.add_node", json!({"name": "n"}))
            .unwrap();
        let id: Value = serde_json::from_str(&result_text(&res)).unwrap();
        let id = id["id"].as_str().unwrap().to_string();

        bridge
            .call_tool(
                "scene.set_translation",
                json!({"id": id, "x": 1.0, "y": 2.0, "z": 3.0}),
            )
            .unwrap();

        let content = bridge.read_resource("scene://nodes").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["nodes"][0]["translation"], json!([1.0, 2.0, 3.0]));
    }

    #[test]
    fn remove_node_empties_scene() {
        let bridge = make_bridge();
        let res = bridge
            .call_tool("scene.add_node", json!({"name": "n"}))
            .unwrap();
        let id: Value = serde_json::from_str(&result_text(&res)).unwrap();
        let id = id["id"].as_str().unwrap().to_string();

        bridge
            .call_tool("scene.remove_node", json!({"id": id}))
            .unwrap();
        let overview = bridge.read_resource("scene://overview").unwrap();
        let v: Value = serde_json::from_str(&overview.text).unwrap();
        assert_eq!(v["node_count"], 0);
    }

    #[test]
    fn reparent_rejects_cycle() {
        let bridge = make_bridge();
        let a = {
            let r = bridge
                .call_tool("scene.add_node", json!({"name": "a"}))
                .unwrap();
            let v: Value = serde_json::from_str(&result_text(&r)).unwrap();
            v["id"].as_str().unwrap().to_string()
        };
        let b = {
            let r = bridge
                .call_tool("scene.add_node", json!({"name": "b", "parent": a}))
                .unwrap();
            let v: Value = serde_json::from_str(&result_text(&r)).unwrap();
            v["id"].as_str().unwrap().to_string()
        };
        // Making `a` a child of its descendant `b` must fail.
        let res = bridge.call_tool("scene.reparent", json!({"id": a, "parent": b}));
        assert!(res.is_err());
    }

    #[test]
    fn invalid_id_errors() {
        let bridge = make_bridge();
        let res = bridge.call_tool("scene.remove_node", json!({"id": "999:0"}));
        assert!(res.is_err());
    }
}
