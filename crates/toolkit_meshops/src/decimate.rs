//! Mesh simplification by vertex clustering (Rossignac-Borrel).
//!
//! Space is divided into a uniform grid; all vertices in a cell collapse to
//! their average. Simple, robust, and predictable — a good default. (Quadric
//! error decimation can be added later for higher quality at equal triangle
//! budgets; this keeps the module dependency-light.)

use std::collections::HashMap;

use glam::{Vec2, Vec3};
use toolkit_geometry::{Mesh, Vertex};

/// Simplify `mesh` by clustering vertices into a grid `resolution` cells across
/// the longest bounding-box axis. Higher `resolution` = more detail retained.
pub fn decimate_grid(mesh: &Mesh, resolution: u32) -> Mesh {
    let resolution = resolution.max(1);
    let bb = mesh.bounding_box();
    let extent = (bb.max - bb.min).max(Vec3::splat(1e-6));
    let cell = extent.max_element() / resolution as f32;
    let inv = 1.0 / cell.max(1e-9);

    let cell_of = |p: Vec3| -> (i64, i64, i64) {
        (
            ((p.x - bb.min.x) * inv).floor() as i64,
            ((p.y - bb.min.y) * inv).floor() as i64,
            ((p.z - bb.min.z) * inv).floor() as i64,
        )
    };

    // Accumulate average position/normal/uv per cell.
    struct Cluster {
        index: u32,
        pos: Vec3,
        normal: Vec3,
        uv: Vec2,
        count: f32,
    }
    let mut clusters: HashMap<(i64, i64, i64), Cluster> = HashMap::new();
    let mut remap = vec![0u32; mesh.vertices.len()];
    let mut next_index = 0u32;

    for (i, v) in mesh.vertices.iter().enumerate() {
        let key = cell_of(v.position_vec3());
        let entry = clusters.entry(key).or_insert_with(|| {
            let idx = next_index;
            next_index += 1;
            Cluster {
                index: idx,
                pos: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                count: 0.0,
            }
        });
        entry.pos += v.position_vec3();
        entry.normal += v.normal_vec3();
        entry.uv += v.uv_vec2();
        entry.count += 1.0;
        remap[i] = entry.index;
    }

    // Build the reduced vertex list ordered by cluster index.
    let mut new_vertices: Vec<Vertex> = vec![Vertex::position_only(Vec3::ZERO); next_index as usize];
    for c in clusters.values() {
        new_vertices[c.index as usize] = Vertex::new(
            c.pos / c.count,
            (c.normal / c.count).normalize_or_zero(),
            c.uv / c.count,
        );
    }

    let mut new_indices = Vec::new();
    for tri in mesh.indices.chunks_exact(3) {
        let a = remap[tri[0] as usize];
        let b = remap[tri[1] as usize];
        let c = remap[tri[2] as usize];
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
    fn reduces_sphere_complexity() {
        let sphere = Mesh::uv_sphere(1.0, 48, 32);
        let before_v = sphere.vertex_count();
        let before_t = sphere.triangle_count();
        let simplified = decimate_grid(&sphere, 8);
        assert!(simplified.vertex_count() < before_v, "vertices not reduced");
        assert!(simplified.triangle_count() < before_t, "triangles not reduced");
        assert!(simplified.vertex_count() > 0);
    }

    #[test]
    fn preserves_rough_bounds() {
        let sphere = Mesh::uv_sphere(2.0, 48, 32);
        let simplified = decimate_grid(&sphere, 10);
        let bb = simplified.bounding_box();
        // Still roughly a radius-2 sphere.
        assert!(bb.max.x > 1.5 && bb.max.x <= 2.01);
        assert!(bb.min.x < -1.5 && bb.min.x >= -2.01);
    }

    #[test]
    fn higher_resolution_keeps_more() {
        let sphere = Mesh::uv_sphere(1.0, 48, 32);
        let coarse = decimate_grid(&sphere, 4);
        let fine = decimate_grid(&sphere, 16);
        assert!(fine.vertex_count() >= coarse.vertex_count());
    }
}
