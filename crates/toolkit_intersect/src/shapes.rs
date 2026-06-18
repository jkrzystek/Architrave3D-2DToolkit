//! Lightweight bounding shapes used by the intersection and closest-point
//! routines. [`toolkit_geometry::Aabb`] and `Ray` cover the rest.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// A plane in the form `normal · p + d = 0`, with a unit `normal`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    /// Build a plane from a (not necessarily unit) normal and a point on it.
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize();
        Self {
            normal: n,
            d: -n.dot(point),
        }
    }

    /// Signed distance from `p` to the plane; positive on the normal's side.
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.d
    }

    /// Normalise the plane so `normal` is unit length (scales `d` to match).
    pub fn normalized(self) -> Self {
        let len = self.normal.length();
        if len <= f32::EPSILON {
            self
        } else {
            Self {
                normal: self.normal / len,
                d: self.d / len,
            }
        }
    }
}

/// A sphere defined by a centre and radius.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
}

/// A finite line segment between two endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub a: Vec3,
    pub b: Vec3,
}

impl Segment {
    pub fn new(a: Vec3, b: Vec3) -> Self {
        Self { a, b }
    }
}

/// A capsule: all points within `radius` of the core segment `a`–`b`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Capsule {
    pub a: Vec3,
    pub b: Vec3,
    pub radius: f32,
}

impl Capsule {
    pub fn new(a: Vec3, b: Vec3, radius: f32) -> Self {
        Self { a, b, radius }
    }

    pub fn segment(&self) -> Segment {
        Segment::new(self.a, self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_signed_distance() {
        let p = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        assert!((p.signed_distance(Vec3::new(0.0, 3.0, 0.0)) - 3.0).abs() < 1e-6);
        assert!((p.signed_distance(Vec3::new(0.0, -2.0, 0.0)) + 2.0).abs() < 1e-6);
    }

    #[test]
    fn plane_normalizes() {
        let p = Plane {
            normal: Vec3::new(0.0, 2.0, 0.0),
            d: -4.0,
        }
        .normalized();
        assert!((p.normal.length() - 1.0).abs() < 1e-6);
        // Point y=2 lies on the plane.
        assert!(p.signed_distance(Vec3::new(0.0, 2.0, 0.0)).abs() < 1e-6);
    }

    #[test]
    fn serde_roundtrip() {
        let c = Capsule::new(Vec3::ZERO, Vec3::Y, 0.5);
        let json = serde_json::to_string(&c).unwrap();
        let back: Capsule = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
