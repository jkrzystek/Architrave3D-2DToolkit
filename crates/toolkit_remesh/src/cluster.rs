//! Vertex-clustering remesh: snap vertices to a uniform grid and weld each cell
//! into a single representative vertex.
//!
//! Unlike QEM decimation (which preserves shape carefully), clustering is a fast,
//! unconditionally robust way to unify resolution and merge nearby/duplicate
//! geometry — useful as a cleanup pass or to coarsen scanned/booleaned meshes.

use std::collections::HashMap;

use glam::{Vec2, Vec3};
use toolkit_geometry::{Mesh, Vertex};

/// Remesh by clustering vertices into grid cells of side `cell_size`. Each
/// cell's representative is the average of the vertices that fell into it.
pub fn cluster_remesh(mesh: &Mesh, cell_size: f32) -> Mesh {
    let cell = cell_size.max(1e-6);
    let key = |p: Vec3| -> (i64, i64, i64) {
        (
            (p.x / cell).floor() as i64,
            (p.y / cell).floor() as i64,
            (p.z / cell).floor() as i64,
        )
    };

    // Assign each original vertex to a cluster, accumulating averages.
    let mut cluster_of: Vec<usize> = vec![0; mesh.vertices.len()];
    let mut cells: HashMap<(i64, i64, i64), usize> = HashMap::new();
    let mut sums: Vec<Vec3> = Vec::new();
    let mut counts: Vec<u32> = Vec::new();
    for (i, v) in mesh.vertices.iter().enumerate() {
        let p = v.position_vec3();
        let k = key(p);
        let idx = *cells.entry(k).or_insert_with(|| {
            sums.push(Vec3::ZERO);
            counts.push(0);
            sums.len() - 1
        });
        sums[idx] += p;
        counts[idx] += 1;
        cluster_of[i] = idx;
    }

    let positions: Vec<Vec3> = sums
        .iter()
        .zip(&counts)
        .map(|(s, &c)| *s / c.max(1) as f32)
        .collect();

    // Remap faces; drop those that collapse to a degenerate triangle.
    let mut indices: Vec<u32> = Vec::new();
    for t in mesh.indices.chunks_exact(3) {
        let a = cluster_of[t[0] as usize];
        let b = cluster_of[t[1] as usize];
        let c = cluster_of[t[2] as usize];
        if a == b || b == c || a == c {
            continue;
        }
        indices.extend_from_slice(&[a as u32, b as u32, c as u32]);
    }

    // Smooth normals.
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for t in indices.chunks_exact(3) {
        let (i, j, k) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let n = (positions[j] - positions[i]).cross(positions[k] - positions[i]);
        normals[i] += n;
        normals[j] += n;
        normals[k] += n;
    }
    let verts: Vec<Vertex> = positions
        .iter()
        .enumerate()
        .map(|(i, &p)| Vertex::new(p, normals[i].normalize_or_zero(), Vec2::ZERO))
        .collect();
    Mesh::with_vertices("remeshed", verts, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clustering_reduces_vertices() {
        let sphere = Mesh::uv_sphere(1.0, 32, 24);
        let before = sphere.vertex_count();
        let remeshed = cluster_remesh(&sphere, 0.4);
        assert!(remeshed.vertex_count() < before);
        assert!(remeshed.triangle_count() > 0);
    }

    #[test]
    fn coarse_cell_collapses_small_mesh() {
        // Three vertices within one cell merge to a single vertex -> no faces.
        use glam::Vec3;
        use toolkit_geometry::Vertex;
        let verts = vec![
            Vertex::position_only(Vec3::new(0.1, 0.1, 0.1)),
            Vertex::position_only(Vec3::new(0.2, 0.1, 0.1)),
            Vertex::position_only(Vec3::new(0.15, 0.2, 0.1)),
        ];
        let tri = Mesh::with_vertices("tri", verts, vec![0, 1, 2]);
        let remeshed = cluster_remesh(&tri, 1.0);
        assert_eq!(remeshed.vertex_count(), 1);
        assert_eq!(remeshed.triangle_count(), 0);
    }

    #[test]
    fn fine_cell_preserves_distinct_vertices() {
        let cube = Mesh::cube(2.0);
        // Cell smaller than the gap between corners keeps them separate.
        let remeshed = cluster_remesh(&cube, 0.1);
        assert_eq!(remeshed.triangle_count(), cube.triangle_count());
    }
}
