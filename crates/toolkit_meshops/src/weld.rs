//! Vertex welding: merge near-coincident vertices and drop the resulting
//! degenerate triangles. Turns "triangle soup" (e.g. the primitive cube's 24
//! split vertices) into shared topology.

use std::collections::HashMap;

use toolkit_geometry::Mesh;

/// Merge vertices whose positions fall within `epsilon` of each other (snapped
/// to an `epsilon` grid). Indices are remapped and triangles that collapse to a
/// line/point are removed. Vertex attributes of the first occurrence are kept.
pub fn weld_vertices(mesh: &Mesh, epsilon: f32) -> Mesh {
    let inv = 1.0 / epsilon.max(1e-9);
    let key = |p: [f32; 3]| -> (i64, i64, i64) {
        (
            (p[0] * inv).round() as i64,
            (p[1] * inv).round() as i64,
            (p[2] * inv).round() as i64,
        )
    };

    let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut new_vertices = Vec::new();
    let mut remap = vec![0u32; mesh.vertices.len()];

    for (i, v) in mesh.vertices.iter().enumerate() {
        let k = key(v.position);
        let new_index = *map.entry(k).or_insert_with(|| {
            new_vertices.push(*v);
            (new_vertices.len() - 1) as u32
        });
        remap[i] = new_index;
    }

    let mut new_indices = Vec::with_capacity(mesh.indices.len());
    for tri in mesh.indices.chunks_exact(3) {
        let a = remap[tri[0] as usize];
        let b = remap[tri[1] as usize];
        let c = remap[tri[2] as usize];
        // Skip degenerate triangles (two or more shared vertices).
        if a != b && b != c && a != c {
            new_indices.extend_from_slice(&[a, b, c]);
        }
    }

    Mesh::with_vertices(mesh.name.clone(), new_vertices, new_indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welds_cube_corners() {
        // The primitive cube has 24 vertices (4 per face, unshared).
        let cube = Mesh::cube(2.0);
        assert_eq!(cube.vertex_count(), 24);
        let welded = weld_vertices(&cube, 1e-4);
        // Should collapse to the 8 geometric corners.
        assert_eq!(welded.vertex_count(), 8);
        // Triangle count is preserved (no faces were degenerate).
        assert_eq!(welded.triangle_count(), 12);
    }

    #[test]
    fn keeps_distinct_vertices() {
        let plane = Mesh::plane(2.0, 2.0, 2);
        let before = plane.vertex_count();
        let welded = weld_vertices(&plane, 1e-5);
        assert_eq!(welded.vertex_count(), before);
    }

    #[test]
    fn removes_degenerate_triangles() {
        // Two vertices at the same spot -> the triangle collapses.
        use glam::{Vec2, Vec3};
        use toolkit_geometry::Vertex;
        let verts = vec![
            Vertex::new(Vec3::ZERO, Vec3::Y, Vec2::ZERO),
            Vertex::new(Vec3::ZERO, Vec3::Y, Vec2::ZERO), // duplicate of 0
            Vertex::new(Vec3::X, Vec3::Y, Vec2::ZERO),
        ];
        let mesh = Mesh::with_vertices("t", verts, vec![0, 1, 2]);
        let welded = weld_vertices(&mesh, 1e-4);
        assert_eq!(welded.triangle_count(), 0);
    }
}
