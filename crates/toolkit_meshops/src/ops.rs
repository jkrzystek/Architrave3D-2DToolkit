//! Assorted mesh operations: winding flip, merge, Laplacian smoothing, stats.

use std::collections::HashSet;

use glam::Vec3;
use toolkit_geometry::{Aabb, Mesh};

use crate::attributes::recompute_normals;

/// Reverse triangle winding (and flip normals) — e.g. to turn a mesh inside-out
/// or fix imported back-faces.
pub fn flip_winding(mesh: &mut Mesh) {
    for tri in mesh.indices.chunks_exact_mut(3) {
        tri.swap(1, 2);
    }
    for v in &mut mesh.vertices {
        v.normal = [-v.normal[0], -v.normal[1], -v.normal[2]];
    }
}

/// Combine several meshes into one, offsetting indices appropriately.
pub fn merge(meshes: &[Mesh], name: impl Into<String>) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for mesh in meshes {
        let offset = vertices.len() as u32;
        vertices.extend_from_slice(&mesh.vertices);
        indices.extend(mesh.indices.iter().map(|&i| i + offset));
    }
    Mesh::with_vertices(name, vertices, indices)
}

/// Summary statistics for a mesh.
#[derive(Clone, Copy, Debug)]
pub struct MeshStats {
    pub vertices: usize,
    pub triangles: usize,
    pub bounds: Aabb,
    pub surface_area: f32,
}

/// Compute vertex/triangle counts, bounding box, and total surface area.
pub fn stats(mesh: &Mesh) -> MeshStats {
    let mut area = 0.0;
    for tri in mesh.indices.chunks_exact(3) {
        let p0 = mesh.vertices[tri[0] as usize].position_vec3();
        let p1 = mesh.vertices[tri[1] as usize].position_vec3();
        let p2 = mesh.vertices[tri[2] as usize].position_vec3();
        area += (p1 - p0).cross(p2 - p0).length() * 0.5;
    }
    MeshStats {
        vertices: mesh.vertex_count(),
        triangles: mesh.triangle_count(),
        bounds: mesh.bounding_box(),
        surface_area: area,
    }
}

/// Laplacian smoothing: move each vertex toward the average of its connected
/// neighbours by `factor` (0..1), for `iterations` passes. Recomputes normals
/// afterwards. Operates on a copy of the positions per pass (Jacobi style).
pub fn laplacian_smooth(mesh: &mut Mesh, iterations: usize, factor: f32) {
    let n = mesh.vertices.len();
    if n == 0 {
        return;
    }
    // Build neighbour sets from edges.
    let mut neighbours: Vec<HashSet<u32>> = vec![HashSet::new(); n];
    for tri in mesh.indices.chunks_exact(3) {
        for k in 0..3 {
            let a = tri[k];
            let b = tri[(k + 1) % 3];
            neighbours[a as usize].insert(b);
            neighbours[b as usize].insert(a);
        }
    }

    for _ in 0..iterations {
        let positions: Vec<Vec3> = mesh.vertices.iter().map(|v| v.position_vec3()).collect();
        for i in 0..n {
            if neighbours[i].is_empty() {
                continue;
            }
            let mut avg = Vec3::ZERO;
            for &j in &neighbours[i] {
                avg += positions[j as usize];
            }
            avg /= neighbours[i].len() as f32;
            let smoothed = positions[i].lerp(avg, factor);
            mesh.vertices[i].position = smoothed.into();
        }
    }
    recompute_normals(mesh);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_reverses_winding() {
        let mut cube = Mesh::cube(1.0);
        let before = cube.indices.clone();
        flip_winding(&mut cube);
        // First triangle's second/third indices are swapped.
        assert_eq!(cube.indices[0], before[0]);
        assert_eq!(cube.indices[1], before[2]);
        assert_eq!(cube.indices[2], before[1]);
    }

    #[test]
    fn merge_concatenates() {
        let a = Mesh::cube(1.0);
        let b = Mesh::cube(1.0);
        let merged = merge(&[a.clone(), b.clone()], "both");
        assert_eq!(merged.vertex_count(), a.vertex_count() * 2);
        assert_eq!(merged.triangle_count(), a.triangle_count() * 2);
    }

    #[test]
    fn stats_cube() {
        let cube = Mesh::cube(2.0);
        let s = stats(&cube);
        assert_eq!(s.vertices, 24);
        assert_eq!(s.triangles, 12);
        // Surface area of a 2x2x2 cube = 6 * (2*2) = 24.
        assert!((s.surface_area - 24.0).abs() < 1e-3, "area = {}", s.surface_area);
    }

    #[test]
    fn smoothing_shrinks_bumpy_mesh_extent() {
        // A sphere stays roughly spherical but smoothing pulls it slightly inward.
        let mut sphere = Mesh::uv_sphere(1.0, 16, 12);
        let before = stats(&sphere).bounds;
        laplacian_smooth(&mut sphere, 3, 0.5);
        let after = stats(&sphere).bounds;
        let before_ext = (before.max - before.min).length();
        let after_ext = (after.max - after.min).length();
        assert!(after_ext <= before_ext + 1e-4);
    }
}
