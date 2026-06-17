//! Direct projection unwraps — fast, no solver. Good for primitives and as a
//! starting point that the user can relax with LSCM.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn vec(self) -> Vec3 {
        match self {
            Axis::X => Vec3::X,
            Axis::Y => Vec3::Y,
            Axis::Z => Vec3::Z,
        }
    }
}

/// Planar projection: drop the component along `axis` and normalise the result
/// into the unit square.
pub fn project_planar(positions: &[Vec3], axis: Axis) -> Vec<Vec2> {
    let mut uvs: Vec<Vec2> = positions
        .iter()
        .map(|p| match axis {
            Axis::X => Vec2::new(p.z, p.y),
            Axis::Y => Vec2::new(p.x, p.z),
            Axis::Z => Vec2::new(p.x, p.y),
        })
        .collect();
    crate::lscm::normalize_to_unit_square(&mut uvs);
    uvs
}

/// Cylindrical projection around `axis`: `u` wraps the angle, `v` follows the
/// height along the axis (normalised).
pub fn project_cylindrical(positions: &[Vec3], axis: Axis) -> Vec<Vec2> {
    let a = axis.vec();
    // Build two perpendicular axes to measure the angle in.
    let (e0, e1) = perpendicular_basis(a);

    let mut heights = Vec::with_capacity(positions.len());
    let mut uvs: Vec<Vec2> = positions
        .iter()
        .map(|p| {
            let x = p.dot(e0);
            let y = p.dot(e1);
            let h = p.dot(a);
            heights.push(h);
            let u = (y.atan2(x) / (2.0 * PI)) + 0.5;
            Vec2::new(u, h)
        })
        .collect();

    // Normalise the height component to [0,1].
    let (mut hmin, mut hmax) = (f32::INFINITY, f32::NEG_INFINITY);
    for &h in &heights {
        hmin = hmin.min(h);
        hmax = hmax.max(h);
    }
    let span = (hmax - hmin).max(1e-12);
    for uv in &mut uvs {
        uv.y = (uv.y - hmin) / span;
    }
    uvs
}

/// Spherical projection about `center`: longitude -> `u`, latitude -> `v`.
pub fn project_spherical(positions: &[Vec3], center: Vec3) -> Vec<Vec2> {
    positions
        .iter()
        .map(|p| {
            let d = (*p - center).normalize_or_zero();
            let u = (d.z.atan2(d.x) / (2.0 * PI)) + 0.5;
            let v = (d.y.clamp(-1.0, 1.0).asin() / PI) + 0.5;
            Vec2::new(u, v)
        })
        .collect()
}

/// Box projection: each vertex is projected on the plane of its dominant axis
/// (by position relative to `center`). Fast and seam-tolerant for blocky models.
pub fn project_box(positions: &[Vec3], center: Vec3) -> Vec<Vec2> {
    let mut uvs: Vec<Vec2> = positions
        .iter()
        .map(|p| {
            let d = *p - center;
            let abs = d.abs();
            if abs.x >= abs.y && abs.x >= abs.z {
                Vec2::new(d.z, d.y)
            } else if abs.y >= abs.x && abs.y >= abs.z {
                Vec2::new(d.x, d.z)
            } else {
                Vec2::new(d.x, d.y)
            }
        })
        .collect();
    crate::lscm::normalize_to_unit_square(&mut uvs);
    uvs
}

fn perpendicular_basis(axis: Vec3) -> (Vec3, Vec3) {
    let a = axis.normalize_or_zero();
    let helper = if a.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let e0 = helper.cross(a).normalize_or_zero();
    let e1 = a.cross(e0).normalize_or_zero();
    (e0, e1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planar_drops_axis_and_fits_unit() {
        let pos = vec![
            Vec3::new(-2.0, 5.0, -2.0),
            Vec3::new(2.0, 5.0, -2.0),
            Vec3::new(2.0, 5.0, 2.0),
            Vec3::new(-2.0, 5.0, 2.0),
        ];
        let uvs = project_planar(&pos, Axis::Y);
        for uv in &uvs {
            assert!((0.0..=1.0).contains(&uv.x));
            assert!((0.0..=1.0).contains(&uv.y));
        }
        // The four corners should map to the four corners of the square.
        let area = (uvs[1] - uvs[0]).perp_dot(uvs[3] - uvs[0]).abs();
        assert!(area > 0.5);
    }

    #[test]
    fn cylindrical_u_in_range() {
        let pos: Vec<Vec3> = (0..8)
            .map(|i| {
                let a = 2.0 * PI * i as f32 / 8.0;
                Vec3::new(a.cos(), i as f32 * 0.1, a.sin())
            })
            .collect();
        let uvs = project_cylindrical(&pos, Axis::Y);
        for uv in &uvs {
            assert!((0.0..=1.0).contains(&uv.x));
            assert!((0.0..=1.0).contains(&uv.y));
        }
    }

    #[test]
    fn spherical_maps_poles() {
        let pos = vec![Vec3::Y, Vec3::NEG_Y, Vec3::X];
        let uvs = project_spherical(&pos, Vec3::ZERO);
        // North pole -> v≈1, south pole -> v≈0.
        assert!(uvs[0].y > 0.9);
        assert!(uvs[1].y < 0.1);
    }

    #[test]
    fn box_projection_fits_unit() {
        let pos = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(-1.0, 0.0, 0.0),
        ];
        let uvs = project_box(&pos, Vec3::ZERO);
        for uv in &uvs {
            assert!((-1e-4..=1.0001).contains(&uv.x));
            assert!((-1e-4..=1.0001).contains(&uv.y));
        }
    }
}
