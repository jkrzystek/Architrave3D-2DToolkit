use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{json, Value};
use toolkit_input::{StrokeStabilizer, Stroke};

use crate::error::{BridgeError, BridgeResult};
use crate::provider::*;

/// AI bridge for input configuration and stroke telemetry.
///
/// Exposes stabilizer config and stroke metadata — NOT raw input sample streams.
pub struct InputBridge {
    stabilizer: Arc<RwLock<StrokeStabilizer>>,
    current_stroke: Arc<RwLock<Option<Stroke>>>,
}

impl InputBridge {
    pub fn new(
        stabilizer: Arc<RwLock<StrokeStabilizer>>,
        current_stroke: Arc<RwLock<Option<Stroke>>>,
    ) -> Self {
        Self {
            stabilizer,
            current_stroke,
        }
    }
}

impl AiProvider for InputBridge {
    fn namespace(&self) -> &str {
        "input"
    }

    fn description(&self) -> &str {
        "Input stabilizer configuration and stroke metadata. Exposes settings and \
         stroke summaries (point count, duration, bounds), not raw sample streams."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![
            ResourceDescriptor::json(
                "input://stabilizer",
                "Stabilizer Config",
                "Current stroke stabilizer settings (spring constant, damping, dead zone)",
            ),
            ResourceDescriptor::json(
                "input://stroke",
                "Current Stroke",
                "Active stroke metadata (point count, duration, bounding box)",
            ),
        ]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "input.set_stabilizer",
                "Configure the stroke stabilizer",
                json!({
                    "type": "object",
                    "properties": {
                        "spring_constant": {"type": "number", "description": "Spring stiffness (higher = snappier)"},
                        "damping": {"type": "number", "description": "Velocity damping (0..1, higher = more damping)"},
                        "dead_zone": {"type": "number", "description": "Minimum movement threshold in pixels"},
                        "enabled": {"type": "boolean", "description": "Toggle stabilizer on/off"},
                    },
                }),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        match uri {
            "input://stabilizer" => {
                let stab = self.stabilizer.read();
                let config = stab.config();
                ResourceContent::json(
                    uri,
                    &json!({
                        "spring_constant": config.spring_constant,
                        "damping": config.damping,
                        "dead_zone": config.dead_zone,
                        "enabled": config.enabled,
                        "is_active": stab.is_active(),
                    }),
                )
            }
            "input://stroke" => {
                let stroke = self.current_stroke.read();
                match stroke.as_ref() {
                    Some(s) => {
                        let (bb_min, bb_max) = s.bounding_box();
                        ResourceContent::json(
                            uri,
                            &json!({
                                "active": true,
                                "point_count": s.point_count(),
                                "duration_ms": s.duration_ms(),
                                "bounding_box": {
                                    "min": [bb_min.x, bb_min.y],
                                    "max": [bb_max.x, bb_max.y],
                                },
                            }),
                        )
                    }
                    None => ResourceContent::json(uri, &json!({"active": false})),
                }
            }
            _ => Err(BridgeError::ResourceNotFound(uri.into())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        match name {
            "input.set_stabilizer" => {
                let mut stab = self.stabilizer.write();
                let mut config = *stab.config();
                if let Some(v) = args.get("spring_constant").and_then(|v| v.as_f64()) {
                    config.spring_constant = v as f32;
                }
                if let Some(v) = args.get("damping").and_then(|v| v.as_f64()) {
                    config.damping = v as f32;
                }
                if let Some(v) = args.get("dead_zone").and_then(|v| v.as_f64()) {
                    config.dead_zone = v as f32;
                }
                if let Some(v) = args.get("enabled").and_then(|v| v.as_bool()) {
                    config.enabled = v;
                }
                stab.set_config(config);
                ToolResult::success_json(&json!({
                    "spring_constant": config.spring_constant,
                    "damping": config.damping,
                    "dead_zone": config.dead_zone,
                    "enabled": config.enabled,
                }))
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_input::StabilizerConfig;

    fn make_bridge() -> InputBridge {
        let stab = StrokeStabilizer::new(StabilizerConfig::default());
        InputBridge::new(
            Arc::new(RwLock::new(stab)),
            Arc::new(RwLock::new(None)),
        )
    }

    #[test]
    fn read_stabilizer_config() {
        let bridge = make_bridge();
        let content = bridge.read_resource("input://stabilizer").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert!(v["enabled"].as_bool().unwrap());
    }

    #[test]
    fn read_no_stroke() {
        let bridge = make_bridge();
        let content = bridge.read_resource("input://stroke").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["active"], false);
    }

    #[test]
    fn set_stabilizer() {
        let bridge = make_bridge();
        bridge
            .call_tool("input.set_stabilizer", json!({"spring_constant": 2.0, "enabled": false}))
            .unwrap();
        let config = bridge.stabilizer.read().config().clone();
        assert!((config.spring_constant - 2.0).abs() < 1e-5);
        assert!(!config.enabled);
    }
}
