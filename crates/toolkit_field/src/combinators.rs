//! Composable combinators on scalar [`Field`]s.
//!
//! Each method on [`FieldExt`] consumes a field and returns a new field, so
//! pipelines build up with zero allocation and stay statically typed:
//!
//! ```ignore
//! let terrain = base.add(detail).scale(2.0).clamp(0.0, 10.0);
//! ```

use glam::Vec3;

use crate::field::{Field, VectorField};

/// Sum of two fields.
pub struct AddF<A, B>(pub A, pub B);
impl<A: Field, B: Field> Field for AddF<A, B> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p) + self.1.sample(p)
    }
}

/// Difference of two fields.
pub struct SubF<A, B>(pub A, pub B);
impl<A: Field, B: Field> Field for SubF<A, B> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p) - self.1.sample(p)
    }
}

/// Product of two fields.
pub struct MulF<A, B>(pub A, pub B);
impl<A: Field, B: Field> Field for MulF<A, B> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p) * self.1.sample(p)
    }
}

/// Pointwise minimum (SDF union).
pub struct MinF<A, B>(pub A, pub B);
impl<A: Field, B: Field> Field for MinF<A, B> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p).min(self.1.sample(p))
    }
}

/// Pointwise maximum (SDF intersection).
pub struct MaxF<A, B>(pub A, pub B);
impl<A: Field, B: Field> Field for MaxF<A, B> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p).max(self.1.sample(p))
    }
}

/// Scale the value by a constant.
pub struct ScaleF<A>(pub A, pub f32);
impl<A: Field> Field for ScaleF<A> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p) * self.1
    }
}

/// Negate the value.
pub struct NegF<A>(pub A);
impl<A: Field> Field for NegF<A> {
    fn sample(&self, p: Vec3) -> f32 {
        -self.0.sample(p)
    }
}

/// Absolute value.
pub struct AbsF<A>(pub A);
impl<A: Field> Field for AbsF<A> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p).abs()
    }
}

/// Clamp the value into `[lo, hi]`.
pub struct ClampF<A>(pub A, pub f32, pub f32);
impl<A: Field> Field for ClampF<A> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p).clamp(self.1, self.2)
    }
}

/// Linearly remap `[in0, in1]` to `[out0, out1]` (no clamping).
pub struct RemapF<A> {
    pub field: A,
    pub in0: f32,
    pub in1: f32,
    pub out0: f32,
    pub out1: f32,
}
impl<A: Field> Field for RemapF<A> {
    fn sample(&self, p: Vec3) -> f32 {
        let t = (self.field.sample(p) - self.in0) / (self.in1 - self.in0);
        self.out0 + t * (self.out1 - self.out0)
    }
}

/// Apply an arbitrary function to the value.
pub struct MapF<A, G>(pub A, pub G);
impl<A: Field, G: Fn(f32) -> f32> Field for MapF<A, G> {
    fn sample(&self, p: Vec3) -> f32 {
        (self.1)(self.0.sample(p))
    }
}

/// Translate the field's domain by `offset` (samples `p - offset`).
pub struct TranslateF<A>(pub A, pub Vec3);
impl<A: Field> Field for TranslateF<A> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p - self.1)
    }
}

/// Warp the domain by a vector field: sample at `p + warp(p)`. This is the
/// engine behind domain-warped noise and turbulent SDFs.
pub struct WarpF<A, W>(pub A, pub W);
impl<A: Field, W: VectorField> Field for WarpF<A, W> {
    fn sample(&self, p: Vec3) -> f32 {
        self.0.sample(p + self.1.sample_vec(p))
    }
}

/// Combinator methods available on every [`Field`].
pub trait FieldExt: Field + Sized {
    fn add<B: Field>(self, other: B) -> AddF<Self, B> {
        AddF(self, other)
    }
    fn sub<B: Field>(self, other: B) -> SubF<Self, B> {
        SubF(self, other)
    }
    fn mul<B: Field>(self, other: B) -> MulF<Self, B> {
        MulF(self, other)
    }
    fn min<B: Field>(self, other: B) -> MinF<Self, B> {
        MinF(self, other)
    }
    fn max<B: Field>(self, other: B) -> MaxF<Self, B> {
        MaxF(self, other)
    }
    fn scale(self, factor: f32) -> ScaleF<Self> {
        ScaleF(self, factor)
    }
    fn neg(self) -> NegF<Self> {
        NegF(self)
    }
    fn abs(self) -> AbsF<Self> {
        AbsF(self)
    }
    fn clamp(self, lo: f32, hi: f32) -> ClampF<Self> {
        ClampF(self, lo, hi)
    }
    fn remap(self, in0: f32, in1: f32, out0: f32, out1: f32) -> RemapF<Self> {
        RemapF { field: self, in0, in1, out0, out1 }
    }
    fn map<G: Fn(f32) -> f32>(self, f: G) -> MapF<Self, G> {
        MapF(self, f)
    }
    fn translate(self, offset: Vec3) -> TranslateF<Self> {
        TranslateF(self, offset)
    }
    fn warp<W: VectorField>(self, warp: W) -> WarpF<Self, W> {
        WarpF(self, warp)
    }
}

impl<F: Field> FieldExt for F {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{Constant, Sphere};

    #[test]
    fn add_and_scale() {
        let f = Constant(1.0).add(Constant(2.0)).scale(10.0);
        assert_eq!(f.sample(Vec3::ZERO), 30.0);
    }

    #[test]
    fn min_is_sdf_union() {
        let a = Sphere { center: Vec3::new(-0.5, 0.0, 0.0), radius: 1.0 };
        let b = Sphere { center: Vec3::new(0.5, 0.0, 0.0), radius: 1.0 };
        let union = a.min(b);
        // Origin is inside both -> negative.
        assert!(union.sample(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn clamp_and_remap() {
        let f = Constant(5.0).clamp(0.0, 1.0);
        assert_eq!(f.sample(Vec3::ZERO), 1.0);
        let r = Constant(0.5).remap(0.0, 1.0, 10.0, 20.0);
        assert_eq!(r.sample(Vec3::ZERO), 15.0);
    }

    #[test]
    fn translate_shifts_domain() {
        // A field reading x, translated by +2 on x, reads x-2.
        let f = (|p: Vec3| p.x).translate(Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(f.sample(Vec3::new(5.0, 0.0, 0.0)), 3.0);
    }

    #[test]
    fn warp_offsets_sample_point() {
        // Field reads x; warp adds (10,0,0) so it reads x+10.
        let warp = |_p: Vec3| Vec3::new(10.0, 0.0, 0.0);
        let f = (|p: Vec3| p.x).warp(warp);
        assert_eq!(f.sample(Vec3::new(1.0, 0.0, 0.0)), 11.0);
    }

    #[test]
    fn map_applies_function() {
        let f = Constant(3.0).map(|v| v * v);
        assert_eq!(f.sample(Vec3::ZERO), 9.0);
    }
}
