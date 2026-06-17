//! Wavefront OBJ import/export. A compact, dependency-free implementation
//! covering positions, texture coordinates, normals, polygon faces (fan
//! triangulated), and `o`/`g` object groups.

use std::collections::HashMap;
use std::path::Path;

use glam::{Vec2, Vec3};
use toolkit_geometry::{Mesh, Vertex};

use crate::error::{AssetError, AssetResult};
use crate::scene_import::ImportedScene;

/// Parse OBJ text into an [`ImportedScene`]. Each `o`/`g` group becomes a mesh.
pub fn import_obj_str(text: &str) -> AssetResult<ImportedScene> {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut tex_coords: Vec<Vec2> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();

    let mut meshes: Vec<Mesh> = Vec::new();
    let mut current_name = String::from("object");
    // Dedup (pos, uv, norm) index triples within the current group.
    let mut vertex_map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    macro_rules! flush_group {
        () => {
            if !indices.is_empty() {
                meshes.push(Mesh::with_vertices(
                    current_name.clone(),
                    std::mem::take(&mut vertices),
                    std::mem::take(&mut indices),
                ));
                vertex_map.clear();
            }
        };
    }

    for (line_no, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let tag = parts.next().unwrap();
        match tag {
            "v" => positions.push(parse_vec3(&mut parts, line_no)?),
            "vt" => {
                let u = parse_f32(parts.next(), line_no)?;
                let v = parts.next().map(|s| s.parse::<f32>().unwrap_or(0.0)).unwrap_or(0.0);
                tex_coords.push(Vec2::new(u, v));
            }
            "vn" => normals.push(parse_vec3(&mut parts, line_no)?),
            "o" | "g" => {
                flush_group!();
                current_name = parts.collect::<Vec<_>>().join(" ");
                if current_name.is_empty() {
                    current_name = format!("object{}", meshes.len());
                }
            }
            "f" => {
                let face: Vec<&str> = parts.collect();
                if face.len() < 3 {
                    return Err(AssetError::Parse(format!(
                        "face with < 3 vertices on line {}",
                        line_no + 1
                    )));
                }
                // Resolve each face-vertex to a unique output vertex.
                let mut face_idx = Vec::with_capacity(face.len());
                for token in &face {
                    let key = parse_face_vertex(token, positions.len(), tex_coords.len(), normals.len())?;
                    let idx = *vertex_map.entry(key).or_insert_with(|| {
                        let pos = positions[(key.0 - 1) as usize];
                        let uv = if key.1 > 0 {
                            tex_coords[(key.1 - 1) as usize]
                        } else {
                            Vec2::ZERO
                        };
                        let norm = if key.2 > 0 {
                            normals[(key.2 - 1) as usize]
                        } else {
                            Vec3::ZERO
                        };
                        vertices.push(Vertex::new(pos, norm, uv));
                        (vertices.len() - 1) as u32
                    });
                    face_idx.push(idx);
                }
                // Fan triangulation.
                for i in 1..face_idx.len() - 1 {
                    indices.push(face_idx[0]);
                    indices.push(face_idx[i]);
                    indices.push(face_idx[i + 1]);
                }
            }
            _ => {} // ignore mtllib, usemtl, s, etc.
        }
    }
    flush_group!();

    if meshes.is_empty() {
        return Err(AssetError::Parse("no geometry found".into()));
    }
    Ok(ImportedScene {
        meshes,
        instances: Vec::new(),
    })
}

/// Read an OBJ file from disk.
pub fn import_obj_path(path: impl AsRef<Path>) -> AssetResult<ImportedScene> {
    let text = std::fs::read_to_string(path)?;
    import_obj_str(&text)
}

/// Serialise meshes to OBJ text.
pub fn export_obj(meshes: &[Mesh]) -> String {
    let mut out = String::from("# exported by toolkit_assets\n");
    let mut pos_offset: u32 = 1; // OBJ indices are 1-based.

    for mesh in meshes {
        out.push_str(&format!("o {}\n", sanitize(&mesh.name)));
        for v in &mesh.vertices {
            out.push_str(&format!(
                "v {} {} {}\n",
                v.position[0], v.position[1], v.position[2]
            ));
        }
        for v in &mesh.vertices {
            out.push_str(&format!("vt {} {}\n", v.uv[0], v.uv[1]));
        }
        for v in &mesh.vertices {
            out.push_str(&format!(
                "vn {} {} {}\n",
                v.normal[0], v.normal[1], v.normal[2]
            ));
        }
        for tri in mesh.indices.chunks_exact(3) {
            let a = tri[0] + pos_offset;
            let b = tri[1] + pos_offset;
            let c = tri[2] + pos_offset;
            out.push_str(&format!(
                "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}\n"
            ));
        }
        pos_offset += mesh.vertices.len() as u32;
    }
    out
}

/// Write meshes to an OBJ file on disk.
pub fn export_obj_path(meshes: &[Mesh], path: impl AsRef<Path>) -> AssetResult<()> {
    std::fs::write(path, export_obj(meshes))?;
    Ok(())
}

// -- Parsing helpers ---------------------------------------------------------

fn sanitize(name: &str) -> String {
    let n = name.trim().replace([' ', '\n', '\t'], "_");
    if n.is_empty() {
        "object".into()
    } else {
        n
    }
}

fn parse_f32(token: Option<&str>, line_no: usize) -> AssetResult<f32> {
    token
        .ok_or_else(|| AssetError::Parse(format!("missing number on line {}", line_no + 1)))?
        .parse::<f32>()
        .map_err(|_| AssetError::Parse(format!("bad number on line {}", line_no + 1)))
}

fn parse_vec3<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    line_no: usize,
) -> AssetResult<Vec3> {
    let x = parse_f32(parts.next(), line_no)?;
    let y = parse_f32(parts.next(), line_no)?;
    let z = parse_f32(parts.next(), line_no)?;
    Ok(Vec3::new(x, y, z))
}

/// Resolve one `v/vt/vn` token into 1-based indices, handling negative
/// (relative) references and missing components (returned as 0).
fn parse_face_vertex(
    token: &str,
    n_pos: usize,
    n_uv: usize,
    n_norm: usize,
) -> AssetResult<(i64, i64, i64)> {
    let mut it = token.split('/');
    let v = resolve(it.next(), n_pos)?
        .ok_or_else(|| AssetError::Parse(format!("face vertex missing position: {token}")))?;
    let vt = resolve(it.next(), n_uv)?.unwrap_or(0);
    let vn = resolve(it.next(), n_norm)?.unwrap_or(0);
    Ok((v, vt, vn))
}

fn resolve(token: Option<&str>, count: usize) -> AssetResult<Option<i64>> {
    match token {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => {
            let i: i64 = s
                .parse()
                .map_err(|_| AssetError::Parse(format!("bad index '{s}'")))?;
            let resolved = if i < 0 { count as i64 + 1 + i } else { i };
            if resolved < 1 || resolved > count as i64 {
                return Err(AssetError::Parse(format!("index out of range: {s}")));
            }
            Ok(Some(resolved))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_triangle() {
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 1/1/1 2/2/1 3/3/1
";
        let scene = import_obj_str(obj).unwrap();
        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.meshes[0].triangle_count(), 1);
        assert_eq!(scene.meshes[0].vertex_count(), 3);
    }

    #[test]
    fn quad_face_is_triangulated() {
        let obj = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3 4
";
        let scene = import_obj_str(obj).unwrap();
        assert_eq!(scene.meshes[0].triangle_count(), 2);
    }

    #[test]
    fn groups_become_separate_meshes() {
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
o first
f 1 2 3
o second
f 3 2 1
";
        let scene = import_obj_str(obj).unwrap();
        assert_eq!(scene.meshes.len(), 2);
        assert_eq!(scene.meshes[0].name, "first");
        assert_eq!(scene.meshes[1].name, "second");
    }

    #[test]
    fn negative_indices() {
        let obj = "\
v 0 0 0
v 1 0 0
v 0 1 0
f -3 -2 -1
";
        let scene = import_obj_str(obj).unwrap();
        assert_eq!(scene.meshes[0].triangle_count(), 1);
    }

    #[test]
    fn roundtrip_cube_through_obj() {
        let cube = Mesh::cube(2.0);
        let text = export_obj(std::slice::from_ref(&cube));
        let scene = import_obj_str(&text).unwrap();
        assert_eq!(scene.meshes.len(), 1);
        let m = &scene.meshes[0];
        assert_eq!(m.triangle_count(), 12);
        let bb = m.bounding_box();
        assert!((bb.max.x - 1.0).abs() < 1e-4);
        assert!((bb.min.x + 1.0).abs() < 1e-4);
    }

    #[test]
    fn missing_geometry_errors() {
        assert!(import_obj_str("# empty\n").is_err());
    }
}
