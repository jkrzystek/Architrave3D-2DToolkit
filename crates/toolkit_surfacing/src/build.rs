//! Shared mesh-assembly helpers: build a [`Mesh`] from a grid of cross-section
//! rings, with smooth normals computed from the generated triangles.

use glam::{Vec2, Vec3};
use toolkit_geometry::{Mesh, Vertex};

/// Assemble a mesh from positions, UVs, and triangle indices, computing smooth
/// (area-weighted) per-vertex normals.
pub fn finish_mesh(
    positions: Vec<Vec3>,
    uvs: Vec<Vec2>,
    indices: Vec<u32>,
    name: impl Into<String>,
) -> Mesh {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for tri in indices.chunks_exact(3) {
        let (i, j, k) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let n = (positions[j] - positions[i]).cross(positions[k] - positions[i]);
        normals[i] += n;
        normals[j] += n;
        normals[k] += n;
    }
    let verts: Vec<Vertex> = positions
        .iter()
        .enumerate()
        .map(|(i, &p)| Vertex::new(p, normals[i].normalize_or_zero(), uvs[i]))
        .collect();
    Mesh::with_vertices(name, verts, indices)
}

/// Build a surface from a sequence of `rings` (cross-sections along the `v`
/// direction), each with the same number of points (the `u` direction).
/// `wrap_u` closes the profile loop (tube walls); the path is left open.
pub fn surface_from_grid(rings: &[Vec<Vec3>], wrap_u: bool, name: impl Into<String>) -> Mesh {
    let v_count = rings.len();
    if v_count < 2 || rings[0].len() < 2 {
        return Mesh::new(name);
    }
    let u_count = rings[0].len();

    let mut positions = Vec::with_capacity(v_count * u_count);
    let mut uvs = Vec::with_capacity(v_count * u_count);
    for (vi, ring) in rings.iter().enumerate() {
        debug_assert_eq!(ring.len(), u_count, "all rings must have equal length");
        for (ui, &p) in ring.iter().enumerate() {
            positions.push(p);
            uvs.push(Vec2::new(
                ui as f32 / (u_count - 1).max(1) as f32,
                vi as f32 / (v_count - 1) as f32,
            ));
        }
    }

    let idx = |vi: usize, ui: usize| (vi * u_count + ui) as u32;
    let u_edges = if wrap_u { u_count } else { u_count - 1 };
    let mut indices = Vec::new();
    for vi in 0..v_count - 1 {
        for ui in 0..u_edges {
            let u0 = ui;
            let u1 = (ui + 1) % u_count;
            let a = idx(vi, u0);
            let b = idx(vi, u1);
            let c = idx(vi + 1, u1);
            let d = idx(vi + 1, u0);
            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }

    finish_mesh(positions, uvs, indices, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_counts() {
        // Two rings of 4 points, wrapped -> a quad tube: 8 verts, 8 quads = 16 tris.
        let r0 = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let r1: Vec<Vec3> = r0.iter().map(|p| *p + Vec3::Z).collect();
        let m = surface_from_grid(&[r0, r1], true, "tube");
        assert_eq!(m.vertex_count(), 8);
        assert_eq!(m.triangle_count(), 8); // 4 wrapped columns * 1 row * 2 tris
    }
}
