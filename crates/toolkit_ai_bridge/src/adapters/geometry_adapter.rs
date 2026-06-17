use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{json, Value};
use toolkit_geometry::{Bvh, Mesh, Ray};

use crate::error::{BridgeError, BridgeResult};
use crate::provider::*;

/// Holds a mesh and its precomputed BVH for AI access.
pub struct MeshEntry {
    pub name: String,
    pub mesh: Mesh,
    pub bvh: Bvh,
}

/// AI bridge adapter for geometry and spatial queries.
///
/// Exposes mesh metadata (vertex/triangle counts, bounding boxes) and
/// raycasting — NOT raw vertex/index arrays.
pub struct GeometryBridge {
    meshes: Arc<RwLock<Vec<MeshEntry>>>,
}

impl GeometryBridge {
    pub fn new(meshes: Arc<RwLock<Vec<MeshEntry>>>) -> Self {
        Self { meshes }
    }

    fn mesh_summary(entry: &MeshEntry) -> Value {
        let bb = entry.mesh.bounding_box();
        json!({
            "name": entry.name,
            "vertex_count": entry.mesh.vertices.len(),
            "triangle_count": entry.mesh.triangle_count(),
            "bounding_box": {
                "min": [bb.min.x, bb.min.y, bb.min.z],
                "max": [bb.max.x, bb.max.y, bb.max.z],
            },
        })
    }
}

impl AiProvider for GeometryBridge {
    fn namespace(&self) -> &str {
        "geometry"
    }

    fn description(&self) -> &str {
        "Mesh information and spatial queries. Provides mesh stats and raycasting — \
         does not expose raw vertex/index buffers."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![ResourceDescriptor::json(
            "geometry://meshes",
            "Mesh List",
            "Summary of all loaded meshes (name, vertex count, triangle count, bounding box)",
        )]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "geometry.create_primitive",
                "Create a primitive mesh (cube, plane, sphere)",
                json!({
                    "type": "object",
                    "properties": {
                        "shape": {"type": "string", "enum": ["cube", "plane", "sphere"]},
                        "size": {"type": "number", "default": 1.0, "description": "Size/radius"},
                        "name": {"type": "string", "description": "Optional name for the mesh"},
                    },
                    "required": ["shape"]
                }),
            ),
            ToolDescriptor::new(
                "geometry.raycast",
                "Cast a ray against a named mesh and return hit info",
                json!({
                    "type": "object",
                    "properties": {
                        "mesh_name": {"type": "string"},
                        "origin": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                        "direction": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
                    },
                    "required": ["mesh_name", "origin", "direction"]
                }),
            ),
            ToolDescriptor::new(
                "geometry.remove_mesh",
                "Remove a mesh by name",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                    },
                    "required": ["name"]
                }),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        match uri {
            "geometry://meshes" => {
                let meshes = self.meshes.read();
                let summaries: Vec<Value> = meshes.iter().map(Self::mesh_summary).collect();
                ResourceContent::json(uri, &json!({ "meshes": summaries, "count": summaries.len() }))
            }
            _ => Err(BridgeError::ResourceNotFound(uri.into())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        match name {
            "geometry.create_primitive" => {
                let shape = args.get("shape").and_then(|v| v.as_str()).unwrap_or("cube");
                let size = args.get("size").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                let mesh_name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(shape)
                    .to_string();

                let mesh = match shape {
                    "cube" => Mesh::cube(size),
                    "plane" => Mesh::plane(size, size, 1),
                    "sphere" => Mesh::uv_sphere(size, 32, 16),
                    _ => {
                        return Err(BridgeError::InvalidArguments(
                            format!("unknown shape: {shape}. Use cube, plane, or sphere"),
                        ))
                    }
                };
                let bvh = Bvh::build(&mesh);
                let summary = json!({
                    "name": mesh_name,
                    "vertex_count": mesh.vertices.len(),
                    "triangle_count": mesh.triangle_count(),
                });
                self.meshes.write().push(MeshEntry {
                    name: mesh_name,
                    mesh,
                    bvh,
                });
                ToolResult::success_json(&summary)
            }
            "geometry.raycast" => {
                let mesh_name = args
                    .get("mesh_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'mesh_name'".into()))?;
                let origin = extract_vec3(&args, "origin")?;
                let direction = extract_vec3(&args, "direction")?;

                let meshes = self.meshes.read();
                let entry = meshes
                    .iter()
                    .find(|e| e.name == mesh_name)
                    .ok_or_else(|| {
                        BridgeError::InvalidArguments(format!("mesh '{mesh_name}' not found"))
                    })?;

                let ray = Ray::new(origin, direction);
                match entry.bvh.intersect(&ray, &entry.mesh) {
                    Some(hit) => ToolResult::success_json(&json!({
                        "hit": true,
                        "distance": hit.t,
                        "position": [hit.position.x, hit.position.y, hit.position.z],
                        "normal": [hit.normal.x, hit.normal.y, hit.normal.z],
                        "uv": [hit.uv.x, hit.uv.y],
                        "triangle_index": hit.triangle_index,
                    })),
                    None => ToolResult::success_json(&json!({
                        "hit": false,
                        "message": "Ray did not intersect mesh",
                    })),
                }
            }
            "geometry.remove_mesh" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BridgeError::InvalidArguments("missing 'name'".into()))?;
                let mut meshes = self.meshes.write();
                let before = meshes.len();
                meshes.retain(|e| e.name != name);
                if meshes.len() < before {
                    ToolResult::success_json(&json!({"removed": name}))
                } else {
                    Ok(ToolResult::error(format!("mesh '{name}' not found")))
                }
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

fn extract_vec3(args: &Value, key: &str) -> BridgeResult<glam::Vec3> {
    let arr = args
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| BridgeError::InvalidArguments(format!("missing '{key}' array")))?;
    if arr.len() < 3 {
        return Err(BridgeError::InvalidArguments(format!("{key} needs 3 elements")));
    }
    Ok(glam::Vec3::new(
        arr[0].as_f64().unwrap_or(0.0) as f32,
        arr[1].as_f64().unwrap_or(0.0) as f32,
        arr[2].as_f64().unwrap_or(0.0) as f32,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bridge() -> GeometryBridge {
        GeometryBridge::new(Arc::new(RwLock::new(Vec::new())))
    }

    #[test]
    fn create_primitive_and_list() {
        let bridge = make_bridge();
        bridge
            .call_tool("geometry.create_primitive", json!({"shape": "cube", "size": 2.0, "name": "my_cube"}))
            .unwrap();

        let content = bridge.read_resource("geometry://meshes").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["count"], 1);
        assert_eq!(v["meshes"][0]["name"], "my_cube");
        assert_eq!(v["meshes"][0]["triangle_count"], 12);
    }

    #[test]
    fn raycast_hits_cube() {
        let bridge = make_bridge();
        bridge
            .call_tool("geometry.create_primitive", json!({"shape": "cube", "size": 2.0, "name": "c"}))
            .unwrap();

        let result = bridge
            .call_tool(
                "geometry.raycast",
                json!({
                    "mesh_name": "c",
                    "origin": [0.0, 0.0, 5.0],
                    "direction": [0.0, 0.0, -1.0],
                }),
            )
            .unwrap();
        assert!(!result.is_error);
    }

    #[test]
    fn remove_mesh() {
        let bridge = make_bridge();
        bridge
            .call_tool("geometry.create_primitive", json!({"shape": "plane", "name": "p"}))
            .unwrap();
        bridge.call_tool("geometry.remove_mesh", json!({"name": "p"})).unwrap();
        assert_eq!(bridge.meshes.read().len(), 0);
    }
}
