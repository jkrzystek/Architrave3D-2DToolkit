//! Catmull-Rom splines: a smooth curve that interpolates (passes through) all
//! its control points — convenient for paths and hand-placed waypoints.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Evaluate one uniform Catmull-Rom segment between `p1` and `p2`, using `p0`
/// and `p3` as the neighbouring tangent points. `t ∈ [0, 1]`.
pub fn segment(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// A Catmull-Rom spline through a sequence of points.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatmullRom {
    pub points: Vec<Vec3>,
}

impl CatmullRom {
    pub fn new(points: Vec<Vec3>) -> Self {
        Self { points }
    }

    /// Evaluate at `t ∈ [0, 1]` across the whole spline (passes through every
    /// control point at `t = k / (n - 1)`).
    pub fn evaluate(&self, t: f32) -> Vec3 {
        let n = self.points.len();
        if n == 0 {
            return Vec3::ZERO;
        }
        if n == 1 {
            return self.points[0];
        }
        let spans = n - 1;
        let scaled = (t.clamp(0.0, 1.0)) * spans as f32;
        let mut k = scaled.floor() as usize;
        if k >= spans {
            k = spans - 1;
        }
        let local = scaled - k as f32;

        let p1 = self.points[k];
        let p2 = self.points[k + 1];
        let p0 = if k == 0 { p1 } else { self.points[k - 1] };
        let p3 = if k + 2 < n { self.points[k + 2] } else { p2 };
        segment(p0, p1, p2, p3, local)
    }

    /// Sample into a polyline with `segments_per_span` samples between each pair
    /// of control points.
    pub fn tessellate(&self, segments_per_span: usize) -> Vec<Vec3> {
        let n = self.points.len();
        if n < 2 {
            return self.points.clone();
        }
        let spans = n - 1;
        let total = spans * segments_per_span.max(1);
        (0..=total)
            .map(|i| self.evaluate(i as f32 / total as f32))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts() -> Vec<Vec3> {
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 1.0, 0.0),
        ]
    }

    #[test]
    fn interpolates_control_points() {
        let spline = CatmullRom::new(pts());
        let n = pts().len();
        for (k, expected) in pts().iter().enumerate() {
            let t = k as f32 / (n - 1) as f32;
            let p = spline.evaluate(t);
            assert!((p - *expected).length() < 1e-4, "at {k}: {p:?} vs {expected:?}");
        }
    }

    #[test]
    fn endpoints_exact() {
        let spline = CatmullRom::new(pts());
        assert!((spline.evaluate(0.0) - pts()[0]).length() < 1e-6);
        assert!((spline.evaluate(1.0) - pts()[3]).length() < 1e-6);
    }

    #[test]
    fn tessellate_count() {
        let spline = CatmullRom::new(pts());
        // 3 spans * 8 + 1
        assert_eq!(spline.tessellate(8).len(), 3 * 8 + 1);
    }

    #[test]
    fn single_point_is_constant() {
        let spline = CatmullRom::new(vec![Vec3::new(5.0, 5.0, 5.0)]);
        assert_eq!(spline.evaluate(0.7), Vec3::new(5.0, 5.0, 5.0));
    }
}
