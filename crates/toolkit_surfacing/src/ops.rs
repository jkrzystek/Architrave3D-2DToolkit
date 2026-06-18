//! The four surfacing operators: [`extrude`], [`revolve`], [`loft`], [`sweep`].

use glam::{Quat, Vec2, Vec3};
use toolkit_geometry::Mesh;
use toolkit_triangulate::triangulate;

use crate::build::{finish_mesh, surface_from_grid};

/// Extrude a closed 2D `profile` (in the XY plane, CCW) along +Z by `depth`,
/// producing side walls and, if `caps`, triangulated end faces.
pub fn extrude(profile: &[Vec2], depth: f32, caps: bool) -> Mesh {
    let n = profile.len();
    if n < 3 {
        return Mesh::new("extrude");
    }
    let mut positions: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<Vec2> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Side rings: bottom (z=0) then top (z=depth), sharing the profile's order.
    for p in profile {
        positions.push(Vec3::new(p.x, p.y, 0.0));
        uvs.push(Vec2::new(0.0, 0.0));
    }
    for p in profile {
        positions.push(Vec3::new(p.x, p.y, depth));
        uvs.push(Vec2::new(1.0, 1.0));
    }
    for i in 0..n {
        let i1 = (i + 1) % n;
        let (b0, b1, t0, t1) = (i as u32, i1 as u32, (n + i) as u32, (n + i1) as u32);
        indices.extend_from_slice(&[b0, b1, t1, b0, t1, t0]);
    }

    if caps {
        let tris = triangulate(profile);
        // Bottom cap (faces -Z): wind reversed.
        let base = positions.len() as u32;
        for p in profile {
            positions.push(Vec3::new(p.x, p.y, 0.0));
            uvs.push(Vec2::new(p.x, p.y));
        }
        for t in &tris {
            indices.extend_from_slice(&[base + t[0] as u32, base + t[2] as u32, base + t[1] as u32]);
        }
        // Top cap (faces +Z).
        let base = positions.len() as u32;
        for p in profile {
            positions.push(Vec3::new(p.x, p.y, depth));
            uvs.push(Vec2::new(p.x, p.y));
        }
        for t in &tris {
            indices.extend_from_slice(&[base + t[0] as u32, base + t[1] as u32, base + t[2] as u32]);
        }
    }

    finish_mesh(positions, uvs, indices, "extrude")
}

/// Revolve a `profile` of `(radius, height)` points around the Y axis through
/// `angle` radians using `segments` angular steps. A full `2π` angle yields a
/// closed solid of revolution.
pub fn revolve(profile: &[Vec2], segments: u32, angle: f32) -> Mesh {
    let segments = segments.max(2);
    if profile.len() < 2 {
        return Mesh::new("revolve");
    }
    let mut rings: Vec<Vec<Vec3>> = Vec::with_capacity(segments as usize + 1);
    for s in 0..=segments {
        let theta = angle * s as f32 / segments as f32;
        let (sin, cos) = theta.sin_cos();
        let ring = profile
            .iter()
            .map(|p| Vec3::new(p.x * cos, p.y, p.x * sin))
            .collect();
        rings.push(ring);
    }
    // u = along the profile (open); v = around the axis.
    surface_from_grid(&rings, false, "revolve")
}

/// Loft a surface through a sequence of cross-`sections`, each with the same
/// number of points. `closed_profile` connects each section's last point back to
/// its first (closed tubes); the section sequence itself is left open.
pub fn loft(sections: &[Vec<Vec3>], closed_profile: bool) -> Mesh {
    surface_from_grid(sections, closed_profile, "loft")
}

/// Unit tangents along a path (central differences, one-sided at the ends).
fn tangents(path: &[Vec3]) -> Vec<Vec3> {
    let n = path.len();
    (0..n)
        .map(|i| {
            let t = if n == 1 {
                Vec3::Z
            } else if i == 0 {
                path[1] - path[0]
            } else if i == n - 1 {
                path[n - 1] - path[n - 2]
            } else {
                path[i + 1] - path[i - 1]
            };
            t.normalize_or_zero()
        })
        .collect()
}

/// Sweep a 2D `profile` along a 3D `path` using rotation-minimizing
/// (parallel-transport) frames, so the profile does not twist. `closed_profile`
/// closes the swept tube around its cross-section.
pub fn sweep(profile: &[Vec2], path: &[Vec3], closed_profile: bool) -> Mesh {
    if path.len() < 2 || profile.len() < 2 {
        return Mesh::new("sweep");
    }
    let tans = tangents(path);

    // Initial frame: a normal perpendicular to the first tangent.
    let seed = if tans[0].x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let mut nrm = (seed - tans[0] * seed.dot(tans[0])).normalize_or_zero();
    let mut bin = tans[0].cross(nrm).normalize_or_zero();

    let mut rings: Vec<Vec<Vec3>> = Vec::with_capacity(path.len());
    for i in 0..path.len() {
        if i > 0 {
            // Transport the frame from tangent i-1 to tangent i.
            let (t0, t1) = (tans[i - 1], tans[i]);
            let axis = t0.cross(t1);
            if axis.length() > 1e-6 {
                let angle = t0.dot(t1).clamp(-1.0, 1.0).acos();
                let q = Quat::from_axis_angle(axis.normalize(), angle);
                nrm = q * nrm;
            }
            nrm = (nrm - t1 * t1.dot(nrm)).normalize_or_zero();
            bin = t1.cross(nrm).normalize_or_zero();
        }
        let ring = profile
            .iter()
            .map(|p| path[i] + nrm * p.x + bin * p.y)
            .collect();
        rings.push(ring);
    }
    surface_from_grid(&rings, closed_profile, "sweep")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn square() -> Vec<Vec2> {
        vec![
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ]
    }

    #[test]
    fn extrude_with_caps_is_closed_box() {
        let m = extrude(&square(), 2.0, true);
        // Sides: 8 verts; caps: 2 * 4 verts = 8; total 16.
        assert_eq!(m.vertex_count(), 16);
        // Sides 8 tris + caps 2*2 tris = 12 (a box).
        assert_eq!(m.triangle_count(), 12);
    }

    #[test]
    fn extrude_without_caps_only_walls() {
        let m = extrude(&square(), 1.0, false);
        assert_eq!(m.vertex_count(), 8);
        assert_eq!(m.triangle_count(), 8);
    }

    #[test]
    fn revolve_full_circle_is_round() {
        // A vertical segment at radius 1 revolved fully -> a cylinder surface.
        let profile = vec![Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0)];
        let m = revolve(&profile, 16, TAU);
        assert!(m.vertex_count() > 0);
        let bb = m.bounding_box();
        // Radius ~1 in x and z.
        assert!((bb.max.x - 1.0).abs() < 0.1 && (bb.min.x + 1.0).abs() < 0.1);
    }

    #[test]
    fn loft_between_two_rings() {
        let r0 = vec![Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y];
        let r1: Vec<Vec3> = r0.iter().map(|p| *p + Vec3::Z * 2.0).collect();
        let m = loft(&[r0, r1], true);
        assert_eq!(m.vertex_count(), 8);
        assert_eq!(m.triangle_count(), 8);
    }

    #[test]
    fn sweep_along_straight_path_keeps_profile_size() {
        let profile = square();
        let path = vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 10.0)];
        let m = sweep(&profile, &path, true);
        // 3 rings * 4 profile points.
        assert_eq!(m.vertex_count(), 12);
        // The swept square should keep ~2-unit cross-section width.
        let bb = m.bounding_box();
        assert!((bb.max.x - 1.0).abs() < 1e-4 && (bb.min.x + 1.0).abs() < 1e-4);
    }
}
