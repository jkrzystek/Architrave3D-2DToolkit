//! B-spline curves (De Boor evaluation).

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::knot::{clamped_uniform_knots, domain, find_span};

/// A B-spline curve: control points, a knot vector, and a degree.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BSplineCurve {
    pub control: Vec<Vec3>,
    pub knots: Vec<f32>,
    pub degree: usize,
}

impl BSplineCurve {
    /// Build a clamped, uniform B-spline through the given control points. The
    /// degree is reduced if there are too few points.
    pub fn new(control: Vec<Vec3>, degree: usize) -> Self {
        let degree = degree.min(control.len().saturating_sub(1)).max(1);
        let knots = clamped_uniform_knots(control.len(), degree);
        Self {
            control,
            knots,
            degree,
        }
    }

    /// Build with an explicit knot vector (length must be `control + degree + 1`).
    pub fn with_knots(control: Vec<Vec3>, knots: Vec<f32>, degree: usize) -> Self {
        Self {
            control,
            knots,
            degree,
        }
    }

    pub fn domain(&self) -> (f32, f32) {
        domain(&self.knots, self.degree)
    }

    /// Evaluate the curve at parameter `u` (clamped to the domain) via De Boor.
    pub fn evaluate(&self, u: f32) -> Vec3 {
        let n = self.control.len() - 1;
        let p = self.degree;
        let (lo, hi) = self.domain();
        let u = u.clamp(lo, hi);
        let span = find_span(n, p, u, &self.knots);

        // Working set of affected control points.
        let mut d: Vec<Vec3> = (0..=p).map(|i| self.control[span - p + i]).collect();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = span - p + j;
                let denom = self.knots[i + p - r + 1] - self.knots[i];
                let alpha = if denom.abs() < 1e-9 {
                    0.0
                } else {
                    (u - self.knots[i]) / denom
                };
                d[j] = d[j - 1].lerp(d[j], alpha);
            }
        }
        d[p]
    }

    /// Sample into a polyline of `segments + 1` points across the domain.
    pub fn tessellate(&self, segments: usize) -> Vec<Vec3> {
        let segments = segments.max(1);
        let (lo, hi) = self.domain();
        (0..=segments)
            .map(|i| {
                let t = i as f32 / segments as f32;
                self.evaluate(lo + (hi - lo) * t)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamped_passes_through_ends() {
        let ctrl = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(3.0, 2.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
        ];
        let curve = BSplineCurve::new(ctrl.clone(), 3);
        assert!((curve.evaluate(0.0) - ctrl[0]).length() < 1e-5);
        assert!((curve.evaluate(1.0) - ctrl[3]).length() < 1e-5);
    }

    #[test]
    fn degree_one_is_polyline() {
        let ctrl = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
        ];
        let curve = BSplineCurve::new(ctrl.clone(), 1);
        // Linear B-spline interpolates control points; midpoint of first segment.
        let mid = curve.evaluate(0.25);
        assert!((mid - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4, "mid = {mid:?}");
    }

    #[test]
    fn stays_within_control_hull_bounds() {
        let ctrl = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 5.0, 0.0),
            Vec3::new(2.0, 5.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
        ];
        let curve = BSplineCurve::new(ctrl, 3);
        for i in 0..=20 {
            let p = curve.evaluate(i as f32 / 20.0);
            // Convex-hull property: within the bounding box of the controls.
            assert!(p.x >= -1e-4 && p.x <= 3.0 + 1e-4);
            assert!(p.y >= -1e-4 && p.y <= 5.0 + 1e-4);
        }
    }

    #[test]
    fn tessellate_count() {
        let curve = BSplineCurve::new(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::ONE],
            3,
        );
        assert_eq!(curve.tessellate(16).len(), 17);
    }
}
