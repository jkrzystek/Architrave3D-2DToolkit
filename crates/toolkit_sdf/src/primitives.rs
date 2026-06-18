//! Signed-distance primitives and the [`Sdf`] trait. Distance functions return
//! the signed distance from a point to the surface (negative inside).
//!
//! Both raw functions (`sd_*`) and a small object tree ([`Sdf`] implementors)
//! are provided: use the functions for ad-hoc math, the tree for CSG.

use glam::{Vec2, Vec3};

/// Anything that can report the signed distance from a point to its surface.
pub trait Sdf {
    fn distance(&self, p: Vec3) -> f32;
}

impl Sdf for Box<dyn Sdf> {
    fn distance(&self, p: Vec3) -> f32 {
        self.as_ref().distance(p)
    }
}

// -- Raw distance functions --------------------------------------------------

pub fn sd_sphere(p: Vec3, radius: f32) -> f32 {
    p.length() - radius
}

pub fn sd_box(p: Vec3, half_extents: Vec3) -> f32 {
    let q = p.abs() - half_extents;
    q.max(Vec3::ZERO).length() + q.x.max(q.y.max(q.z)).min(0.0)
}

pub fn sd_round_box(p: Vec3, half_extents: Vec3, radius: f32) -> f32 {
    sd_box(p, half_extents) - radius
}

pub fn sd_torus(p: Vec3, major: f32, minor: f32) -> f32 {
    let q = Vec2::new(Vec2::new(p.x, p.z).length() - major, p.y);
    q.length() - minor
}

pub fn sd_plane(p: Vec3, normal: Vec3, height: f32) -> f32 {
    p.dot(normal.normalize_or_zero()) + height
}

pub fn sd_capsule(p: Vec3, a: Vec3, b: Vec3, radius: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    (pa - ba * h).length() - radius
}

pub fn sd_cylinder(p: Vec3, radius: f32, half_height: f32) -> f32 {
    let d = Vec2::new(Vec2::new(p.x, p.z).length() - radius, p.y.abs() - half_height);
    d.x.max(d.y).min(0.0) + d.max(Vec2::ZERO).length()
}

// -- Primitive objects -------------------------------------------------------

pub struct Sphere {
    pub radius: f32,
}
impl Sdf for Sphere {
    fn distance(&self, p: Vec3) -> f32 {
        sd_sphere(p, self.radius)
    }
}

pub struct BoxSdf {
    pub half_extents: Vec3,
}
impl Sdf for BoxSdf {
    fn distance(&self, p: Vec3) -> f32 {
        sd_box(p, self.half_extents)
    }
}

pub struct Torus {
    pub major: f32,
    pub minor: f32,
}
impl Sdf for Torus {
    fn distance(&self, p: Vec3) -> f32 {
        sd_torus(p, self.major, self.minor)
    }
}

pub struct Plane {
    pub normal: Vec3,
    pub height: f32,
}
impl Sdf for Plane {
    fn distance(&self, p: Vec3) -> f32 {
        sd_plane(p, self.normal, self.height)
    }
}

pub struct Capsule {
    pub a: Vec3,
    pub b: Vec3,
    pub radius: f32,
}
impl Sdf for Capsule {
    fn distance(&self, p: Vec3) -> f32 {
        sd_capsule(p, self.a, self.b, self.radius)
    }
}

pub struct Cylinder {
    pub radius: f32,
    pub half_height: f32,
}
impl Sdf for Cylinder {
    fn distance(&self, p: Vec3) -> f32 {
        sd_cylinder(p, self.radius, self.half_height)
    }
}

/// Approximate surface normal via the gradient of the field (central
/// differences). Works for any [`Sdf`].
pub fn sdf_normal(sdf: &dyn Sdf, p: Vec3, eps: f32) -> Vec3 {
    let dx = sdf.distance(p + Vec3::new(eps, 0.0, 0.0)) - sdf.distance(p - Vec3::new(eps, 0.0, 0.0));
    let dy = sdf.distance(p + Vec3::new(0.0, eps, 0.0)) - sdf.distance(p - Vec3::new(0.0, eps, 0.0));
    let dz = sdf.distance(p + Vec3::new(0.0, 0.0, eps)) - sdf.distance(p - Vec3::new(0.0, 0.0, eps));
    Vec3::new(dx, dy, dz).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_distance_signs() {
        assert!((sd_sphere(Vec3::new(2.0, 0.0, 0.0), 2.0)).abs() < 1e-6); // on surface
        assert!(sd_sphere(Vec3::ZERO, 2.0) < 0.0); // inside
        assert!(sd_sphere(Vec3::new(5.0, 0.0, 0.0), 2.0) > 0.0); // outside
    }

    #[test]
    fn box_distance() {
        let b = Vec3::splat(1.0);
        assert!(sd_box(Vec3::ZERO, b) < 0.0);
        assert!((sd_box(Vec3::new(1.0, 0.0, 0.0), b)).abs() < 1e-6);
        assert!((sd_box(Vec3::new(2.0, 0.0, 0.0), b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn torus_on_surface() {
        // Major 2, minor 0.5: the point (2.5,0,0) is on the outer surface.
        assert!((sd_torus(Vec3::new(2.5, 0.0, 0.0), 2.0, 0.5)).abs() < 1e-6);
    }

    #[test]
    fn normal_points_outward_on_sphere() {
        let s = Sphere { radius: 1.0 };
        let n = sdf_normal(&s, Vec3::new(1.0, 0.0, 0.0), 1e-3);
        assert!((n - Vec3::X).length() < 1e-3);
    }
}
