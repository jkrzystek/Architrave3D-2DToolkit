//! Bézier curves of arbitrary degree (de Casteljau evaluation).

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// A Bézier curve defined by its control polygon. Degree = `control.len() - 1`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bezier {
    pub control: Vec<Vec3>,
}

impl Bezier {
    pub fn new(control: Vec<Vec3>) -> Self {
        Self { control }
    }

    /// Convenience constructor for a cubic Bézier.
    pub fn cubic(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Self {
        Self {
            control: vec![p0, p1, p2, p3],
        }
    }

    pub fn degree(&self) -> usize {
        self.control.len().saturating_sub(1)
    }

    /// Evaluate the curve at `t ∈ [0, 1]` via de Casteljau's algorithm.
    pub fn evaluate(&self, t: f32) -> Vec3 {
        if self.control.is_empty() {
            return Vec3::ZERO;
        }
        let mut pts = self.control.clone();
        let n = pts.len();
        for r in 1..n {
            for i in 0..n - r {
                pts[i] = pts[i].lerp(pts[i + 1], t);
            }
        }
        pts[0]
    }

    /// First derivative (tangent, unnormalised) at `t`. The derivative of a
    /// degree-`n` Bézier is a degree-`(n-1)` Bézier over the difference points.
    pub fn derivative(&self, t: f32) -> Vec3 {
        let n = self.control.len();
        if n < 2 {
            return Vec3::ZERO;
        }
        let deg = (n - 1) as f32;
        let diffs: Vec<Vec3> = self
            .control
            .windows(2)
            .map(|w| (w[1] - w[0]) * deg)
            .collect();
        Bezier::new(diffs).evaluate(t)
    }

    /// Sample the curve into a polyline of `segments + 1` points.
    pub fn tessellate(&self, segments: usize) -> Vec<Vec3> {
        let segments = segments.max(1);
        (0..=segments)
            .map(|i| self.evaluate(i as f32 / segments as f32))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_through_endpoints() {
        let b = Bezier::cubic(Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::ONE);
        assert!((b.evaluate(0.0) - Vec3::ZERO).length() < 1e-6);
        assert!((b.evaluate(1.0) - Vec3::ONE).length() < 1e-6);
    }

    #[test]
    fn linear_bezier_is_lerp() {
        let b = Bezier::new(vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)]);
        assert!((b.evaluate(0.5) - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn derivative_of_line_is_constant() {
        let b = Bezier::new(vec![Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)]);
        let d = b.derivative(0.5);
        assert!((d - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn tessellate_count() {
        let b = Bezier::cubic(Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::ONE);
        assert_eq!(b.tessellate(10).len(), 11);
    }

    #[test]
    fn symmetric_curve_midpoint() {
        // Symmetric control polygon -> midpoint has x = 0.5.
        let b = Bezier::cubic(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!((b.evaluate(0.5).x - 0.5).abs() < 1e-6);
    }
}
