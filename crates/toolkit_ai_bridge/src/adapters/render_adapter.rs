use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{json, Value};
use toolkit_render::camera::{Camera, OrbitController};

use crate::error::{BridgeError, BridgeResult};
use crate::provider::*;

/// AI bridge adapter for camera and viewport control.
///
/// Exposes camera position, orientation, and projection settings.
/// Does NOT expose raw framebuffers or GPU handles.
pub struct CameraBridge {
    camera: Arc<RwLock<Camera>>,
    orbit: Arc<RwLock<OrbitController>>,
}

impl CameraBridge {
    pub fn new(camera: Arc<RwLock<Camera>>, orbit: Arc<RwLock<OrbitController>>) -> Self {
        Self { camera, orbit }
    }
}

impl AiProvider for CameraBridge {
    fn namespace(&self) -> &str {
        "camera"
    }

    fn description(&self) -> &str {
        "3D camera positioning and orbit control. Read camera state or adjust \
         position/target/projection without raw GPU access."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![
            ResourceDescriptor::json(
                "camera://state",
                "Camera State",
                "Current camera position, target, up vector, and projection settings",
            ),
            ResourceDescriptor::json(
                "camera://orbit",
                "Orbit Controller",
                "Orbit controller state: yaw, pitch, distance",
            ),
        ]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "camera.set_position",
                "Set camera world position",
                json!({
                    "type": "object",
                    "properties": {
                        "x": {"type": "number"}, "y": {"type": "number"}, "z": {"type": "number"}
                    },
                    "required": ["x", "y", "z"]
                }),
            ),
            ToolDescriptor::new(
                "camera.set_target",
                "Set the point the camera looks at",
                json!({
                    "type": "object",
                    "properties": {
                        "x": {"type": "number"}, "y": {"type": "number"}, "z": {"type": "number"}
                    },
                    "required": ["x", "y", "z"]
                }),
            ),
            ToolDescriptor::new(
                "camera.orbit",
                "Rotate the orbit controller by delta yaw/pitch (radians)",
                json!({
                    "type": "object",
                    "properties": {
                        "delta_yaw": {"type": "number", "description": "Yaw change in radians"},
                        "delta_pitch": {"type": "number", "description": "Pitch change in radians"},
                    },
                    "required": ["delta_yaw", "delta_pitch"]
                }),
            ),
            ToolDescriptor::new(
                "camera.zoom",
                "Zoom the orbit controller (positive = closer, negative = farther)",
                json!({
                    "type": "object",
                    "properties": {
                        "delta": {"type": "number", "description": "Zoom delta (positive = in)"},
                    },
                    "required": ["delta"]
                }),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        match uri {
            "camera://state" => {
                let cam = self.camera.read();
                let proj_info = match &cam.projection {
                    toolkit_render::camera::Projection::Perspective {
                        fov_y_radians,
                        aspect_ratio,
                        near,
                        far,
                    } => json!({
                        "type": "perspective",
                        "fov_y_degrees": fov_y_radians.to_degrees(),
                        "aspect_ratio": aspect_ratio,
                        "near": near, "far": far,
                    }),
                    toolkit_render::camera::Projection::Orthographic {
                        width,
                        height,
                        near,
                        far,
                    } => json!({
                        "type": "orthographic",
                        "width": width, "height": height,
                        "near": near, "far": far,
                    }),
                };
                ResourceContent::json(
                    uri,
                    &json!({
                        "position": [cam.position.x, cam.position.y, cam.position.z],
                        "target": [cam.target.x, cam.target.y, cam.target.z],
                        "up": [cam.up.x, cam.up.y, cam.up.z],
                        "projection": proj_info,
                    }),
                )
            }
            "camera://orbit" => {
                let orbit = self.orbit.read();
                ResourceContent::json(
                    uri,
                    &json!({
                        "yaw_degrees": orbit.yaw.to_degrees(),
                        "pitch_degrees": orbit.pitch.to_degrees(),
                        "distance": orbit.distance,
                    }),
                )
            }
            _ => Err(BridgeError::ResourceNotFound(uri.into())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        match name {
            "camera.set_position" => {
                let (x, y, z) = extract_xyz(&args)?;
                let mut cam = self.camera.write();
                cam.position = glam::Vec3::new(x, y, z);
                ToolResult::success_json(&json!({
                    "position": [x, y, z],
                }))
            }
            "camera.set_target" => {
                let (x, y, z) = extract_xyz(&args)?;
                let mut cam = self.camera.write();
                cam.target = glam::Vec3::new(x, y, z);
                ToolResult::success_json(&json!({
                    "target": [x, y, z],
                }))
            }
            "camera.orbit" => {
                let dy = args.get("delta_yaw").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let dp = args.get("delta_pitch").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let mut orbit = self.orbit.write();
                orbit.rotate(dy, dp);
                ToolResult::success_json(&json!({
                    "yaw_degrees": orbit.yaw.to_degrees(),
                    "pitch_degrees": orbit.pitch.to_degrees(),
                }))
            }
            "camera.zoom" => {
                let delta = args
                    .get("delta")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'delta'".into()))?
                    as f32;
                let mut orbit = self.orbit.write();
                orbit.zoom(delta);
                ToolResult::success_json(&json!({
                    "distance": orbit.distance,
                }))
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

fn extract_xyz(args: &Value) -> BridgeResult<(f32, f32, f32)> {
    let x = args.get("x").and_then(|v| v.as_f64())
        .ok_or_else(|| BridgeError::InvalidArguments("missing 'x'".into()))? as f32;
    let y = args.get("y").and_then(|v| v.as_f64())
        .ok_or_else(|| BridgeError::InvalidArguments("missing 'y'".into()))? as f32;
    let z = args.get("z").and_then(|v| v.as_f64())
        .ok_or_else(|| BridgeError::InvalidArguments("missing 'z'".into()))? as f32;
    Ok((x, y, z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn make_bridge() -> CameraBridge {
        let cam = Camera::perspective(Vec3::new(0.0, 5.0, 10.0), Vec3::ZERO, 45.0, 16.0 / 9.0);
        let orbit = OrbitController { distance: 10.0, ..Default::default() };
        CameraBridge::new(Arc::new(RwLock::new(cam)), Arc::new(RwLock::new(orbit)))
    }

    #[test]
    fn read_camera_state() {
        let bridge = make_bridge();
        let content = bridge.read_resource("camera://state").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert!(v["position"].is_array());
        assert_eq!(v["projection"]["type"], "perspective");
    }

    #[test]
    fn set_position() {
        let bridge = make_bridge();
        bridge
            .call_tool("camera.set_position", json!({"x": 1.0, "y": 2.0, "z": 3.0}))
            .unwrap();
        let cam = bridge.camera.read();
        assert!((cam.position.x - 1.0).abs() < 1e-5);
    }

    #[test]
    fn orbit_changes_yaw() {
        let bridge = make_bridge();
        let before = bridge.orbit.read().yaw;
        bridge.call_tool("camera.orbit", json!({"delta_yaw": 0.5, "delta_pitch": 0.0})).unwrap();
        let after = bridge.orbit.read().yaw;
        assert!((after - before - 0.5).abs() < 1e-5);
    }
}
