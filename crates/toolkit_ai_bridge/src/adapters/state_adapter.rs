use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{json, Value};
use toolkit_core::LayerKind;
use toolkit_state::Document;

use crate::error::{BridgeError, BridgeResult};
use crate::provider::*;

/// AI bridge adapter for the document/layer state module.
///
/// Exposes document metadata, layer tree structure (not pixel data),
/// and layer manipulation tools.
pub struct DocumentBridge {
    document: Arc<RwLock<Document>>,
}

impl DocumentBridge {
    pub fn new(document: Arc<RwLock<Document>>) -> Self {
        Self { document }
    }

    fn layer_to_json(layer: &toolkit_state::Layer) -> Value {
        json!({
            "id": layer.id.to_string(),
            "name": layer.name,
            "kind": format!("{:?}", layer.kind),
            "opacity": layer.opacity,
            "visible": layer.visible,
            "blend_mode": format!("{:?}", layer.blend_mode),
            "locked": layer.locked,
            "children_count": layer.children.len(),
        })
    }

    fn layer_tree_json(layer: &toolkit_state::Layer) -> Value {
        json!({
            "id": layer.id.to_string(),
            "name": layer.name,
            "kind": format!("{:?}", layer.kind),
            "opacity": layer.opacity,
            "visible": layer.visible,
            "blend_mode": format!("{:?}", layer.blend_mode),
            "locked": layer.locked,
            "children": layer.children.iter()
                .map(Self::layer_tree_json)
                .collect::<Vec<_>>(),
        })
    }

    fn parse_layer_kind(s: &str) -> Result<LayerKind, BridgeError> {
        match s.to_lowercase().as_str() {
            "paint" => Ok(LayerKind::Paint),
            "fill" => Ok(LayerKind::Fill),
            "folder" | "group" => Ok(LayerKind::Folder),
            "mask" => Ok(LayerKind::Mask),
            "adjustment" => Ok(LayerKind::Adjustment),
            _ => Err(BridgeError::InvalidArguments(
                format!("unknown layer kind: {s}. Use: Paint, Fill, Folder, Mask, Adjustment"),
            )),
        }
    }
}

impl AiProvider for DocumentBridge {
    fn namespace(&self) -> &str {
        "document"
    }

    fn description(&self) -> &str {
        "Document and layer tree management. Provides semantic access to layers, \
         properties, and structure — no raw pixel data."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![
            ResourceDescriptor::json(
                "document://info",
                "Document Info",
                "Document name, dimensions, layer count, dirty state",
            ),
            ResourceDescriptor::json(
                "document://layers",
                "Layer Tree",
                "Full layer hierarchy with properties (name, opacity, visibility, blend mode)",
            ),
            ResourceDescriptor::json(
                "document://active_layer",
                "Active Layer",
                "Currently selected layer details",
            ),
        ]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "document.add_layer",
                "Add a new layer to the document",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Layer name"},
                        "kind": {"type": "string", "enum": ["Paint", "Fill", "Folder", "Mask", "Adjustment"], "default": "Paint"},
                    },
                    "required": ["name"]
                }),
            ),
            ToolDescriptor::new(
                "document.remove_layer",
                "Remove a layer by ID",
                json!({
                    "type": "object",
                    "properties": {
                        "layer_id": {"type": "string", "description": "Layer ID to remove"},
                    },
                    "required": ["layer_id"]
                }),
            ),
            ToolDescriptor::new(
                "document.set_layer_opacity",
                "Set a layer's opacity (0.0 to 1.0)",
                json!({
                    "type": "object",
                    "properties": {
                        "layer_id": {"type": "string", "description": "Layer ID"},
                        "opacity": {"type": "number", "minimum": 0.0, "maximum": 1.0},
                    },
                    "required": ["layer_id", "opacity"]
                }),
            ),
            ToolDescriptor::new(
                "document.set_layer_visibility",
                "Toggle layer visibility",
                json!({
                    "type": "object",
                    "properties": {
                        "layer_id": {"type": "string", "description": "Layer ID"},
                        "visible": {"type": "boolean"},
                    },
                    "required": ["layer_id", "visible"]
                }),
            ),
            ToolDescriptor::new(
                "document.rename_layer",
                "Rename a layer",
                json!({
                    "type": "object",
                    "properties": {
                        "layer_id": {"type": "string", "description": "Layer ID"},
                        "name": {"type": "string", "description": "New name"},
                    },
                    "required": ["layer_id", "name"]
                }),
            ),
            ToolDescriptor::new(
                "document.set_active_layer",
                "Set the active (selected) layer",
                json!({
                    "type": "object",
                    "properties": {
                        "layer_id": {"type": "string", "description": "Layer ID to select"},
                    },
                    "required": ["layer_id"]
                }),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        let doc = self.document.read();
        match uri {
            "document://info" => ResourceContent::json(
                uri,
                &json!({
                    "name": doc.name,
                    "width": doc.width,
                    "height": doc.height,
                    "layer_count": doc.layer_count(),
                    "dirty": doc.is_dirty(),
                    "active_layer_id": doc.active_layer_id.map(|id| id.to_string()),
                }),
            ),
            "document://layers" => {
                ResourceContent::json(uri, &Self::layer_tree_json(&doc.root_layer))
            }
            "document://active_layer" => {
                let layer_json = doc
                    .active_layer_id
                    .and_then(|id| doc.find_layer(id))
                    .map(Self::layer_to_json)
                    .unwrap_or(json!(null));
                ResourceContent::json(uri, &layer_json)
            }
            _ => Err(BridgeError::ResourceNotFound(uri.to_string())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        let mut doc = self.document.write();
        match name {
            "document.add_layer" => {
                let layer_name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'name'".into()))?;
                let kind_str = args
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Paint");
                let kind = Self::parse_layer_kind(kind_str)?;
                let id = doc.add_layer(layer_name, kind, None);
                ToolResult::success_json(&json!({
                    "layer_id": id.to_string(),
                    "message": format!("Created {kind_str} layer '{layer_name}'"),
                }))
            }
            "document.remove_layer" => {
                let id_str = args
                    .get("layer_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'layer_id'".into()))?;
                let id = parse_layer_id(id_str)?;
                match doc.remove_layer(id) {
                    Some(layer) => ToolResult::success_json(&json!({
                        "removed": layer.name,
                        "message": format!("Removed layer '{}'", layer.name),
                    })),
                    None => Ok(ToolResult::error(format!("Layer {id_str} not found or is root"))),
                }
            }
            "document.set_layer_opacity" => {
                let id = parse_layer_id_from_args(&args)?;
                let opacity = args
                    .get("opacity")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'opacity'".into()))?
                    as f32;
                match doc.find_layer_mut(id) {
                    Some(layer) => {
                        layer.opacity = opacity.clamp(0.0, 1.0);
                        ToolResult::success_json(&json!({
                            "layer_id": id.to_string(),
                            "opacity": layer.opacity,
                        }))
                    }
                    None => Ok(ToolResult::error("Layer not found")),
                }
            }
            "document.set_layer_visibility" => {
                let id = parse_layer_id_from_args(&args)?;
                let visible = args
                    .get("visible")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'visible'".into()))?;
                match doc.find_layer_mut(id) {
                    Some(layer) => {
                        layer.visible = visible;
                        ToolResult::success_json(&json!({
                            "layer_id": id.to_string(),
                            "visible": visible,
                        }))
                    }
                    None => Ok(ToolResult::error("Layer not found")),
                }
            }
            "document.rename_layer" => {
                let id = parse_layer_id_from_args(&args)?;
                let new_name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'name'".into()))?;
                match doc.find_layer_mut(id) {
                    Some(layer) => {
                        let old = std::mem::replace(&mut layer.name, new_name.into());
                        ToolResult::success_json(&json!({
                            "old_name": old,
                            "new_name": new_name,
                        }))
                    }
                    None => Ok(ToolResult::error("Layer not found")),
                }
            }
            "document.set_active_layer" => {
                let id = parse_layer_id_from_args(&args)?;
                if doc.find_layer(id).is_some() {
                    doc.set_active_layer(id);
                    ToolResult::success_json(&json!({
                        "active_layer_id": id.to_string(),
                    }))
                } else {
                    Ok(ToolResult::error("Layer not found"))
                }
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

fn parse_layer_id(s: &str) -> BridgeResult<toolkit_core::LayerId> {
    let n: u64 = s
        .parse()
        .map_err(|_| BridgeError::InvalidArguments(format!("invalid layer id: {s}")))?;
    Ok(toolkit_core::LayerId::from_raw(n))
}

fn parse_layer_id_from_args(args: &Value) -> BridgeResult<toolkit_core::LayerId> {
    let s = args
        .get("layer_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BridgeError::InvalidArguments("missing 'layer_id'".into()))?;
    parse_layer_id(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bridge() -> DocumentBridge {
        let doc = Document::new("Test", 1920, 1080);
        DocumentBridge::new(Arc::new(RwLock::new(doc)))
    }

    #[test]
    fn read_document_info() {
        let bridge = make_bridge();
        let content = bridge.read_resource("document://info").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["name"], "Test");
        assert_eq!(v["width"], 1920);
        assert_eq!(v["layer_count"], 1);
    }

    #[test]
    fn read_layer_tree() {
        let bridge = make_bridge();
        let content = bridge.read_resource("document://layers").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["name"], "Root");
        assert!(v["children"].is_array());
    }

    #[test]
    fn add_and_list_layer() {
        let bridge = make_bridge();
        let result = bridge
            .call_tool("document.add_layer", json!({"name": "Sky", "kind": "Paint"}))
            .unwrap();
        assert!(!result.is_error);

        let content = bridge.read_resource("document://info").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["layer_count"], 2);
    }

    #[test]
    fn set_opacity() {
        let bridge = make_bridge();
        let result = bridge
            .call_tool("document.add_layer", json!({"name": "L1"}))
            .unwrap();
        let id_val: Value = serde_json::from_str(result_text(&result)).unwrap();
        let layer_id = id_val["layer_id"].as_str().unwrap();

        let result = bridge
            .call_tool(
                "document.set_layer_opacity",
                json!({"layer_id": layer_id, "opacity": 0.5}),
            )
            .unwrap();
        assert!(!result.is_error);
    }

    #[test]
    fn unknown_resource_returns_error() {
        let bridge = make_bridge();
        assert!(bridge.read_resource("document://pixels").is_err());
    }
}

#[cfg(test)]
fn result_text(result: &ToolResult) -> &str {
    match &result.content[0] {
        ToolResultContent::Text { text } => text,
    }
}
