//! The core [`Field`] / [`VectorField`] traits and the simplest concrete fields.
//!
//! A field answers one question: "what is the value here?". Scalar fields map a
//! point to an `f32` (density, signed distance, mask weight, height); vector
//! fields map a point to a `Vec3` (flow, displacement, gradient). Closures
//! implement both traits automatically, so wrapping a noise function, an SDF, or
//! a volume sampler in a field is free.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// A scalar field: a value at every point in space.
pub trait Field {
    /// Evaluate the field at a world point.
    fn sample(&self, p: Vec3) -> f32;
}

/// A vector field: a `Vec3` at every point in space.
pub trait VectorField {
    /// Evaluate the field at a world point.
    fn sample_vec(&self, p: Vec3) -> Vec3;
}

// Closures are fields. This is what makes noise/SDF/volume samplers usable
// directly: `|p| my_volume.sample(p)` is a `Field`.
impl<F: Fn(Vec3) -> f32> Field for F {
    fn sample(&self, p: Vec3) -> f32 {
        self(p)
    }
}

impl<F: Fn(Vec3) -> Vec3> VectorField for F {
    fn sample_vec(&self, p: Vec3) -> Vec3 {
        self(p)
    }
}

/// A constant scalar field (serializable, unlike a closure).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Constant(pub f32);

impl Field for Constant {
    fn sample(&self, _p: Vec3) -> f32 {
        self.0
    }
}

/// Signed-distance field of a sphere centered at `center` with `radius`
/// (negative inside). A handy, serializable concrete field for testing and for
/// building shapes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Field for Sphere {
    fn sample(&self, p: Vec3) -> f32 {
        (p - self.center).length() - self.radius
    }
}

/// Central-difference gradient of a scalar field, returned as a vector field.
///
/// For an SDF this yields the (approximately unit) surface normal direction.
pub fn gradient<F: Field>(field: F, eps: f32) -> impl VectorField {
    move |p: Vec3| {
        let dx = field.sample(p + Vec3::new(eps, 0.0, 0.0)) - field.sample(p - Vec3::new(eps, 0.0, 0.0));
        let dy = field.sample(p + Vec3::new(0.0, eps, 0.0)) - field.sample(p - Vec3::new(0.0, eps, 0.0));
        let dz = field.sample(p + Vec3::new(0.0, 0.0, eps)) - field.sample(p - Vec3::new(0.0, 0.0, eps));
        Vec3::new(dx, dy, dz) / (2.0 * eps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_is_a_field() {
        let f = |p: Vec3| p.x + p.y;
        assert_eq!(f.sample(Vec3::new(1.0, 2.0, 9.0)), 3.0);
    }

    #[test]
    fn constant_field() {
        assert_eq!(Constant(2.5).sample(Vec3::ONE), 2.5);
    }

    #[test]
    fn sphere_sdf_sign() {
        let s = Sphere { center: Vec3::ZERO, radius: 1.0 };
        assert!(s.sample(Vec3::ZERO) < 0.0); // inside
        assert!((s.sample(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-6); // outside
        assert!(s.sample(Vec3::new(1.0, 0.0, 0.0)).abs() < 1e-6); // surface
    }

    #[test]
    fn gradient_points_outward_on_sphere() {
        let s = Sphere { center: Vec3::ZERO, radius: 1.0 };
        let g = gradient(s, 1e-3);
        let n = g.sample_vec(Vec3::new(1.0, 0.0, 0.0));
        assert!((n.normalize() - Vec3::X).length() < 1e-3);
    }

    #[test]
    fn serde_roundtrip() {
        let s = Sphere { center: Vec3::Y, radius: 2.0 };
        let json = serde_json::to_string(&s).unwrap();
        let back: Sphere = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
