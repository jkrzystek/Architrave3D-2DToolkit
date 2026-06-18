//! Polygonize a signed distance field into a triangle mesh using **naive
//! surface nets** (a dual method): one vertex per surface-crossing cell, placed
//! at the averaged zero-crossings, with quads connecting adjacent cells.
//!
//! Surface nets needs no 256-entry triangle table (unlike marching cubes) and
//! produces smoother, lower-triangle meshes — a good default for SDF editing.

use glam::{Vec2, Vec3};
use toolkit_geometry::{Aabb, Mesh, Vertex};

use crate::primitives::{sdf_normal, Sdf};

/// The 8 cube corners as (x, y, z) offsets in `{0, 1}`.
fn corner_offset(c: usize) -> Vec3 {
    Vec3::new(
        (c & 1) as f32,
        ((c >> 1) & 1) as f32,
        ((c >> 2) & 1) as f32,
    )
}

/// Build the 12 cube edges (as corner-index pairs) and the 256-entry mask ->
/// crossed-edges table at runtime (cheap, avoids a big static table).
fn build_tables() -> ([usize; 24], [u32; 256]) {
    let mut cube_edges = [0usize; 24];
    let mut k = 0;
    for i in 0u32..8 {
        for j in [1u32, 2, 4] {
            let p = i ^ j;
            if i <= p {
                cube_edges[k] = i as usize;
                cube_edges[k + 1] = p as usize;
                k += 2;
            }
        }
    }
    let mut edge_table = [0u32; 256];
    for (mask, slot) in edge_table.iter_mut().enumerate() {
        let mut em = 0u32;
        let mut e = 0;
        while e < 24 {
            let a = (mask & (1 << cube_edges[e])) != 0;
            let b = (mask & (1 << cube_edges[e + 1])) != 0;
            if a != b {
                em |= 1 << (e >> 1);
            }
            e += 2;
        }
        *slot = em;
    }
    (cube_edges, edge_table)
}

/// Polygonize `sdf` over `bounds` at `resolution` cells per axis. Returns a
/// triangle [`Mesh`] (empty if the surface doesn't cross the volume). Normals
/// come from the field gradient.
pub fn polygonize(sdf: &dyn Sdf, bounds: &Aabb, resolution: usize) -> Mesh {
    let res = resolution.max(1);
    let n = res + 1;
    let size = bounds.max - bounds.min;
    let step = size / res as f32;
    let (cube_edges, edge_table) = build_tables();

    // Sample the field at every grid corner.
    let sidx = |i: usize, j: usize, k: usize| i + j * n + k * n * n;
    let mut field = vec![0f32; n * n * n];
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                let p = bounds.min + Vec3::new(i as f32, j as f32, k as f32) * step;
                field[sidx(i, j, k)] = sdf.distance(p);
            }
        }
    }

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let cidx = |i: usize, j: usize, k: usize| i + j * res + k * res * res;
    let mut cell_vertex = vec![u32::MAX; res * res * res];
    let normal_eps = step.min_element() * 0.5;

    for k in 0..res {
        for j in 0..res {
            for i in 0..res {
                // Sample the 8 corners and build the inside/outside mask.
                let mut grid = [0f32; 8];
                let mut mask = 0u32;
                for (c, g) in grid.iter_mut().enumerate() {
                    let o = corner_offset(c);
                    *g = field[sidx(i + o.x as usize, j + o.y as usize, k + o.z as usize)];
                    if *g < 0.0 {
                        mask |= 1 << c;
                    }
                }
                let em = edge_table[mask as usize];
                if em == 0 {
                    continue;
                }

                // Vertex = average of the edge zero-crossings (cell-local).
                let mut vsum = Vec3::ZERO;
                let mut count = 0.0;
                for e in 0..12 {
                    if em & (1 << e) == 0 {
                        continue;
                    }
                    let c0 = cube_edges[2 * e];
                    let c1 = cube_edges[2 * e + 1];
                    let (g0, g1) = (grid[c0], grid[c1]);
                    let denom = g0 - g1;
                    let t = if denom.abs() < 1e-12 { 0.5 } else { g0 / denom };
                    vsum += corner_offset(c0).lerp(corner_offset(c1), t);
                    count += 1.0;
                }
                let local = vsum / count;
                let world = bounds.min + (Vec3::new(i as f32, j as f32, k as f32) + local) * step;
                let normal = sdf_normal(sdf, world, normal_eps);
                let vid = vertices.len() as u32;
                vertices.push(Vertex::new(world, normal, Vec2::ZERO));
                cell_vertex[cidx(i, j, k)] = vid;

                // Emit a quad for each axis whose corner-0 edge crosses.
                let corner0_inside = (mask & 1) != 0;
                for d in 0..3 {
                    let corner = 1usize << d; // neighbour corner along this axis
                    let neighbour_inside = (mask & (1 << corner)) != 0;
                    if corner0_inside == neighbour_inside {
                        continue;
                    }
                    // The four cells sharing this edge (need the -1 neighbours).
                    let quad = match d {
                        0 => {
                            if j == 0 || k == 0 {
                                continue;
                            }
                            [
                                cell_vertex[cidx(i, j, k)],
                                cell_vertex[cidx(i, j - 1, k)],
                                cell_vertex[cidx(i, j - 1, k - 1)],
                                cell_vertex[cidx(i, j, k - 1)],
                            ]
                        }
                        1 => {
                            if i == 0 || k == 0 {
                                continue;
                            }
                            [
                                cell_vertex[cidx(i, j, k)],
                                cell_vertex[cidx(i - 1, j, k)],
                                cell_vertex[cidx(i - 1, j, k - 1)],
                                cell_vertex[cidx(i, j, k - 1)],
                            ]
                        }
                        _ => {
                            if i == 0 || j == 0 {
                                continue;
                            }
                            [
                                cell_vertex[cidx(i, j, k)],
                                cell_vertex[cidx(i - 1, j, k)],
                                cell_vertex[cidx(i - 1, j - 1, k)],
                                cell_vertex[cidx(i, j - 1, k)],
                            ]
                        }
                    };
                    if quad.iter().any(|&v| v == u32::MAX) {
                        continue;
                    }
                    let [v0, v1, v2, v3] = quad;
                    // Orient the quad so its front face points outward. Outward
                    // is +axis when corner 0 is inside, -axis otherwise. Pick the
                    // triangle order whose geometric normal agrees.
                    let axis_dir = match d {
                        0 => Vec3::X,
                        1 => Vec3::Y,
                        _ => Vec3::Z,
                    };
                    let desired = if corner0_inside { axis_dir } else { -axis_dir };
                    let p0 = vertices[v0 as usize].position_vec3();
                    let p1 = vertices[v1 as usize].position_vec3();
                    let p2 = vertices[v2 as usize].position_vec3();
                    let geo = (p1 - p0).cross(p2 - p0);
                    if geo.dot(desired) >= 0.0 {
                        indices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
                    } else {
                        indices.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
                    }
                }
            }
        }
    }

    Mesh::with_vertices("sdf_surface", vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::Sphere;

    fn unit_bounds(r: f32) -> Aabb {
        Aabb::new(Vec3::splat(-r * 1.5), Vec3::splat(r * 1.5))
    }

    #[test]
    fn sphere_produces_surface() {
        let s = Sphere { radius: 1.0 };
        let mesh = polygonize(&s, &unit_bounds(1.0), 24);
        assert!(mesh.vertex_count() > 100, "too few vertices");
        assert!(mesh.triangle_count() > 100, "too few triangles");
    }

    #[test]
    fn sphere_vertices_lie_near_surface() {
        let s = Sphere { radius: 1.0 };
        let mesh = polygonize(&s, &unit_bounds(1.0), 32);
        for v in &mesh.vertices {
            let r = v.position_vec3().length();
            // Within roughly one cell of the true radius.
            assert!((r - 1.0).abs() < 0.1, "vertex off-surface: r = {r}");
        }
    }

    #[test]
    fn winding_is_mostly_outward() {
        let s = Sphere { radius: 1.0 };
        let mesh = polygonize(&s, &unit_bounds(1.0), 20);
        let mut agree = 0;
        let mut total = 0;
        for tri in mesh.indices.chunks_exact(3) {
            let p0 = mesh.vertices[tri[0] as usize].position_vec3();
            let p1 = mesh.vertices[tri[1] as usize].position_vec3();
            let p2 = mesh.vertices[tri[2] as usize].position_vec3();
            let geo = (p1 - p0).cross(p2 - p0);
            // The gradient normal at the first vertex should point outward.
            let n = mesh.vertices[tri[0] as usize].normal_vec3();
            if geo.dot(n) > 0.0 {
                agree += 1;
            }
            total += 1;
        }
        // The vast majority of triangles should be wound consistently outward.
        assert!(agree as f32 / total as f32 > 0.8, "winding inconsistent: {agree}/{total}");
    }

    #[test]
    fn empty_volume_makes_empty_mesh() {
        // A sphere far outside the sampled bounds -> no crossings.
        let s = Sphere { radius: 0.1 };
        let bounds = Aabb::new(Vec3::splat(10.0), Vec3::splat(11.0));
        let mesh = polygonize(&s, &bounds, 8);
        assert_eq!(mesh.triangle_count(), 0);
    }
}
