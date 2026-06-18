//! Convert a triangle mesh into a [`Volume`]: a signed-distance field, a solid
//! occupancy field, or a thin surface shell.
//!
//! The signed-distance field is the core result. For each lattice point it finds
//! the unsigned distance to the nearest triangle and signs it by an inside test
//! (parity of ray crossings along a skewed ray). The solid and surface fields
//! are thresholds of the SDF.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use toolkit_geometry::{ray_triangle_intersection, Mesh, Ray};
use toolkit_volume::Volume;

use crate::closest::closest_point_on_triangle;

/// Controls the grid the mesh is sampled onto.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoxelizeConfig {
    /// Number of lattice points along the mesh's longest axis (min 2).
    pub resolution: usize,
    /// World-space margin added around the mesh bounding box on every side.
    pub padding: f32,
}

impl Default for VoxelizeConfig {
    fn default() -> Self {
        Self {
            resolution: 32,
            padding: 0.0,
        }
    }
}

fn triangles(mesh: &Mesh) -> Vec<[Vec3; 3]> {
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            [
                mesh.vertices[t[0] as usize].position_vec3(),
                mesh.vertices[t[1] as usize].position_vec3(),
                mesh.vertices[t[2] as usize].position_vec3(),
            ]
        })
        .collect()
}

fn unsigned_distance(p: Vec3, tris: &[[Vec3; 3]]) -> f32 {
    let mut best = f32::INFINITY;
    for t in tris {
        let cp = closest_point_on_triangle(p, t[0], t[1], t[2]);
        let d = cp.distance_squared(p);
        if d < best {
            best = d;
        }
    }
    best.sqrt()
}

/// Inside test by ray-crossing parity along a slightly skewed ray (the skew
/// avoids the axis-aligned degeneracies of a pure +X ray).
fn is_inside(p: Vec3, tris: &[[Vec3; 3]]) -> bool {
    let ray = Ray::new(p, Vec3::new(0.5773, 0.5575, 0.5967));
    let mut hits = 0;
    for t in tris {
        if let Some((dist, _, _)) = ray_triangle_intersection(&ray, t[0], t[1], t[2]) {
            if dist > 1e-6 {
                hits += 1;
            }
        }
    }
    hits % 2 == 1
}

/// Determine the lattice dimensions, origin, and cell size for a mesh.
fn grid_layout(mesh: &Mesh, cfg: &VoxelizeConfig) -> ([usize; 3], Vec3, f32) {
    let bb = mesh.bounding_box();
    let pad = Vec3::splat(cfg.padding);
    let min = bb.min - pad;
    let max = bb.max + pad;
    let extent = (max - min).max(Vec3::splat(1e-4));
    let res = cfg.resolution.max(2);
    let cell = (extent.max_element() / (res - 1) as f32).max(1e-6);
    let dims = [
        (extent.x / cell).ceil() as usize + 1,
        (extent.y / cell).ceil() as usize + 1,
        (extent.z / cell).ceil() as usize + 1,
    ];
    (dims, min, cell)
}

/// Signed-distance field of a mesh (negative inside, positive outside).
pub fn signed_distance_field(mesh: &Mesh, cfg: &VoxelizeConfig) -> Volume<f32> {
    let tris = triangles(mesh);
    let (dims, origin, cell) = grid_layout(mesh, cfg);
    if tris.is_empty() {
        return Volume::new(dims, origin, Vec3::splat(cell), 0.0);
    }
    Volume::from_fn(dims, origin, Vec3::splat(cell), |[x, y, z]| {
        let p = origin + Vec3::new(x as f32, y as f32, z as f32) * cell;
        let d = unsigned_distance(p, &tris);
        if is_inside(p, &tris) {
            -d
        } else {
            d
        }
    })
}

/// Solid occupancy: `1.0` inside the mesh, `0.0` outside.
pub fn solid(mesh: &Mesh, cfg: &VoxelizeConfig) -> Volume<f32> {
    let sdf = signed_distance_field(mesh, cfg);
    let dims = sdf.size();
    let mut out = Volume::new(dims, sdf.origin(), sdf.cell_size(), 0.0);
    for (dst, &d) in out.as_mut_slice().iter_mut().zip(sdf.as_slice()) {
        *dst = if d <= 0.0 { 1.0 } else { 0.0 };
    }
    out
}

/// Surface shell: `1.0` where the SDF magnitude is within `band`, else `0.0`.
pub fn surface_shell(mesh: &Mesh, cfg: &VoxelizeConfig, band: f32) -> Volume<f32> {
    let sdf = signed_distance_field(mesh, cfg);
    let dims = sdf.size();
    let mut out = Volume::new(dims, sdf.origin(), sdf.cell_size(), 0.0);
    for (dst, &d) in out.as_mut_slice().iter_mut().zip(sdf.as_slice()) {
        *dst = if d.abs() <= band { 1.0 } else { 0.0 };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_sdf_negative_at_center() {
        let cube = Mesh::cube(2.0); // spans [-1, 1]
        let cfg = VoxelizeConfig {
            resolution: 12,
            padding: 0.5,
        };
        let sdf = signed_distance_field(&cube, &cfg);
        // Sample at the world origin (cube center): distance to nearest face ~1.
        let center = sdf.sample(Vec3::ZERO);
        assert!(center < -0.5, "center SDF should be clearly inside: {center}");
        // A point well outside is positive.
        let outside = sdf.sample(Vec3::new(3.0, 0.0, 0.0));
        assert!(outside > 0.0, "outside SDF should be positive: {outside}");
    }

    #[test]
    fn solid_marks_interior() {
        let cube = Mesh::cube(2.0);
        let cfg = VoxelizeConfig {
            resolution: 12,
            padding: 0.5,
        };
        let solid = solid(&cube, &cfg);
        assert_eq!(solid.sample(Vec3::ZERO), 1.0);
        assert_eq!(solid.sample(Vec3::new(3.0, 0.0, 0.0)), 0.0);
    }

    #[test]
    fn surface_shell_thin_band() {
        let cube = Mesh::cube(2.0);
        let cfg = VoxelizeConfig {
            resolution: 16,
            padding: 0.5,
        };
        let shell = surface_shell(&cube, &cfg, 0.3);
        // On a face (x≈1) the shell should be set; deep inside it should not.
        assert_eq!(shell.sample(Vec3::new(1.0, 0.0, 0.0)), 1.0);
        assert_eq!(shell.sample(Vec3::ZERO), 0.0);
    }

    #[test]
    fn empty_mesh_yields_zero_volume() {
        let m = Mesh::new("empty");
        let v = signed_distance_field(&m, &VoxelizeConfig::default());
        assert!(v.as_slice().iter().all(|&d| d == 0.0));
    }
}
