//! glTF 2.0 import (via the `gltf` crate). Reads `.gltf`/`.glb`, including
//! embedded base64 buffers, and flattens the node hierarchy into world-space
//! mesh instances.

use std::path::Path;

use glam::{Mat4, Vec2, Vec3};
use toolkit_geometry::{Mesh, Vertex};
use toolkit_scene::Transform;

use crate::error::{AssetError, AssetResult};
use crate::scene_import::{ImportedScene, MeshInstance};

/// Import glTF from raw bytes (`.glb` or self-contained `.gltf`).
pub fn import_gltf_slice(bytes: &[u8]) -> AssetResult<ImportedScene> {
    let (document, buffers, _images) =
        gltf::import_slice(bytes).map_err(|e| AssetError::Gltf(e.to_string()))?;
    build(document, buffers)
}

/// Import a glTF file from disk (resolves external `.bin` buffers relative to
/// the file).
pub fn import_gltf_path(path: impl AsRef<Path>) -> AssetResult<ImportedScene> {
    let (document, buffers, _images) =
        gltf::import(path).map_err(|e| AssetError::Gltf(e.to_string()))?;
    build(document, buffers)
}

fn build(document: gltf::Document, buffers: Vec<gltf::buffer::Data>) -> AssetResult<ImportedScene> {
    let mut scene = ImportedScene::default();

    for gltf_scene in document.scenes() {
        for node in gltf_scene.nodes() {
            walk_node(&node, Mat4::IDENTITY, &buffers, &mut scene);
        }
    }

    if scene.meshes.is_empty() {
        return Err(AssetError::Gltf("no meshes in document".into()));
    }
    Ok(scene)
}

fn walk_node(
    node: &gltf::Node,
    parent_world: Mat4,
    buffers: &[gltf::buffer::Data],
    out: &mut ImportedScene,
) {
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world = parent_world * local;

    if let Some(mesh) = node.mesh() {
        let node_name = node.name().unwrap_or("node").to_string();
        for (i, primitive) in mesh.primitives().enumerate() {
            if let Some(m) = read_primitive(&primitive, buffers, &node_name, i) {
                let mesh_index = out.meshes.len();
                out.meshes.push(m);
                out.instances.push(MeshInstance {
                    name: node_name.clone(),
                    transform: Transform::from_matrix(&world),
                    mesh_index,
                });
            }
        }
    }

    for child in node.children() {
        walk_node(&child, world, buffers, out);
    }
}

fn read_primitive(
    primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
    base_name: &str,
    prim_index: usize,
) -> Option<Mesh> {
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|d| d.0.as_slice()));

    let positions: Vec<[f32; 3]> = reader.read_positions()?.collect();
    let normals: Option<Vec<[f32; 3]>> = reader.read_normals().map(|n| n.collect());
    let uvs: Option<Vec<[f32; 2]>> = reader
        .read_tex_coords(0)
        .map(|t| t.into_f32().collect());

    let vertices: Vec<Vertex> = positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let normal = normals
                .as_ref()
                .and_then(|n| n.get(i))
                .map(|n| Vec3::from(*n))
                .unwrap_or(Vec3::ZERO);
            let uv = uvs
                .as_ref()
                .and_then(|u| u.get(i))
                .map(|u| Vec2::from(*u))
                .unwrap_or(Vec2::ZERO);
            Vertex::new(Vec3::from(*p), normal, uv)
        })
        .collect();

    let indices: Vec<u32> = match reader.read_indices() {
        Some(i) => i.into_u32().collect(),
        None => (0..vertices.len() as u32).collect(),
    };

    Some(Mesh::with_vertices(
        format!("{base_name}#{prim_index}"),
        vertices,
        indices,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal glTF 2.0 document: one triangle with an embedded base64 buffer.
    fn triangle_gltf() -> String {
        // Buffer: 3 positions (9 f32) followed by 3 indices (u16, padded).
        // Built as base64 of the little-endian bytes.
        // positions: (0,0,0)(1,0,0)(0,1,0); indices: 0,1,2
        let mut bytes: Vec<u8> = Vec::new();
        for f in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        let index_offset = bytes.len();
        for i in [0u16, 1, 2] {
            bytes.extend_from_slice(&i.to_le_bytes());
        }
        // Pad to 4-byte alignment.
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
        let b64 = base64_encode(&bytes);
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "scenes": [{{ "nodes": [0] }}],
  "scene": 0,
  "nodes": [{{ "mesh": 0 }}],
  "meshes": [{{ "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1
  }}] }}],
  "buffers": [{{ "uri": "data:application/octet-stream;base64,{b64}", "byteLength": {len} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36, "target": 34962 }},
    {{ "buffer": 0, "byteOffset": {index_offset}, "byteLength": 6, "target": 34963 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
       "min": [0,0,0], "max": [1,1,0] }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ]
}}"#,
            len = bytes.len()
        )
    }

    // Tiny standalone base64 encoder (avoids adding a dependency for one test).
    fn base64_encode(data: &[u8]) -> String {
        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in data.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
            out.push(TABLE[((n >> 18) & 63) as usize] as char);
            out.push(TABLE[((n >> 12) & 63) as usize] as char);
            if chunk.len() > 1 {
                out.push(TABLE[((n >> 6) & 63) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(TABLE[(n & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }

    #[test]
    fn imports_embedded_triangle() {
        let gltf = triangle_gltf();
        let scene = import_gltf_slice(gltf.as_bytes()).unwrap();
        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.meshes[0].triangle_count(), 1);
        assert_eq!(scene.instances.len(), 1);
    }

    #[test]
    fn import_builds_scene_graph() {
        let gltf = triangle_gltf();
        let imported = import_gltf_slice(gltf.as_bytes()).unwrap();
        let scene = imported.build_scene();
        assert_eq!(scene.len(), 1);
    }
}
