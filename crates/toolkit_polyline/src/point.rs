//! The [`Point`] trait abstracts over `Vec2` and `Vec3` so every polyline
//! operation is written once and works in both 2D and 3D.

use glam::{Vec2, Vec3};

/// A point in 2D or 3D space, with the vector arithmetic polyline ops need.
pub trait Point: Copy {
    fn add(self, other: Self) -> Self;
    fn sub(self, other: Self) -> Self;
    fn scale(self, s: f32) -> Self;
    fn dot(self, other: Self) -> f32;

    fn length(self) -> f32 {
        self.dot(self).max(0.0).sqrt()
    }
    fn distance(self, other: Self) -> f32 {
        self.sub(other).length()
    }
    fn lerp(self, other: Self, t: f32) -> Self {
        self.add(other.sub(self).scale(t))
    }
}

impl Point for Vec2 {
    fn add(self, o: Self) -> Self {
        self + o
    }
    fn sub(self, o: Self) -> Self {
        self - o
    }
    fn scale(self, s: f32) -> Self {
        self * s
    }
    fn dot(self, o: Self) -> f32 {
        Vec2::dot(self, o)
    }
}

impl Point for Vec3 {
    fn add(self, o: Self) -> Self {
        self + o
    }
    fn sub(self, o: Self) -> Self {
        self - o
    }
    fn scale(self, s: f32) -> Self {
        self * s
    }
    fn dot(self, o: Self) -> f32 {
        Vec3::dot(self, o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_and_lerp_2d() {
        let a = Vec2::ZERO;
        let b = Vec2::new(3.0, 4.0);
        assert!((a.distance(b) - 5.0).abs() < 1e-6);
        assert_eq!(a.lerp(b, 0.5), Vec2::new(1.5, 2.0));
    }

    #[test]
    fn distance_3d() {
        assert!((Vec3::ZERO.distance(Vec3::new(2.0, 3.0, 6.0)) - 7.0).abs() < 1e-6);
    }
}
