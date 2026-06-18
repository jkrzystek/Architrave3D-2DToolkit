//! The [`VolumeSample`] trait: cell types that can be linearly blended, which is
//! all trilinear sampling and resampling need.

use glam::{Vec2, Vec3, Vec4};

/// A value that can be linearly interpolated, so it can be trilinearly sampled
/// inside a [`Volume`](crate::Volume).
pub trait VolumeSample: Copy {
    /// The additive identity (used as the default fill / out-of-range value).
    fn zero() -> Self;
    /// Linear blend: `t = 0` returns `self`, `t = 1` returns `other`.
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl VolumeSample for f32 {
    fn zero() -> Self {
        0.0
    }
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl VolumeSample for Vec2 {
    fn zero() -> Self {
        Vec2::ZERO
    }
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl VolumeSample for Vec3 {
    fn zero() -> Self {
        Vec3::ZERO
    }
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl VolumeSample for Vec4 {
    fn zero() -> Self {
        Vec4::ZERO
    }
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_lerp() {
        assert_eq!(2.0_f32.lerp(4.0, 0.5), 3.0);
        assert_eq!(f32::zero(), 0.0);
    }

    #[test]
    fn vec3_lerp() {
        let a = Vec3::ZERO;
        let b = Vec3::new(2.0, 4.0, 6.0);
        assert_eq!(a.lerp(b, 0.5), Vec3::new(1.0, 2.0, 3.0));
    }
}
