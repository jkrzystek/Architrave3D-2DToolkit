//! NURBS curves — B-splines with per-control-point weights, evaluated in
//! homogeneous coordinates. Weights let a NURBS represent conics (circles,
//! ellipses) exactly, which plain B-splines cannot.

use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};

use crate::knot::{clamped_uniform_knots, de_boor4, domain};

/// A NURBS curve: control points, matching weights, a knot vector, and degree.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NurbsCurve {
    pub control: Vec<Vec3>,
    pub weights: Vec<f32>,
    pub knots: Vec<f32>,
    pub degree: usize,
}

impl NurbsCurve {
    /// Clamped, uniform NURBS. `weights` must match `control` in length.
    pub fn new(control: Vec<Vec3>, weights: Vec<f32>, degree: usize) -> Self {
        assert_eq!(control.len(), weights.len(), "control/weight length mismatch");
        let degree = degree.min(control.len().saturating_sub(1)).max(1);
        let knots = clamped_uniform_knots(control.len(), degree);
        Self {
            control,
            weights,
            knots,
            degree,
        }
    }

    /// A NURBS with all weights equal to 1 (equivalent to a B-spline).
    pub fn uniform_weights(control: Vec<Vec3>, degree: usize) -> Self {
        let weights = vec![1.0; control.len()];
        Self::new(control, weights, degree)
    }

    pub fn domain(&self) -> (f32, f32) {
        domain(&self.knots, self.degree)
    }

    fn homogeneous(&self) -> Vec<Vec4> {
        self.control
            .iter()
            .zip(&self.weights)
            .map(|(p, &w)| (*p * w).extend(w))
            .collect()
    }

    /// Evaluate the curve at parameter `u` (clamped to the domain).
    pub fn evaluate(&self, u: f32) -> Vec3 {
        let (lo, hi) = self.domain();
        let u = u.clamp(lo, hi);
        let n = self.control.len() - 1;
        let hpts = self.homogeneous();
        let r = de_boor4(n, self.degree, u, &self.knots, &hpts);
        if r.w.abs() < 1e-9 {
            r.truncate()
        } else {
            r.truncate() / r.w
        }
    }

    /// Sample into a polyline of `segments + 1` points across the domain.
    pub fn tessellate(&self, segments: usize) -> Vec<Vec3> {
        let segments = segments.max(1);
        let (lo, hi) = self.domain();
        (0..=segments)
            .map(|i| self.evaluate(lo + (hi - lo) * (i as f32 / segments as f32)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bspline::BSplineCurve;

    fn ctrl() -> Vec<Vec3> {
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(3.0, 2.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
        ]
    }

    #[test]
    fn unit_weights_match_bspline() {
        let nurbs = NurbsCurve::uniform_weights(ctrl(), 3);
        let bspline = BSplineCurve::new(ctrl(), 3);
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let a = nurbs.evaluate(t);
            let b = bspline.evaluate(t);
            assert!((a - b).length() < 1e-4, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn passes_through_clamped_ends() {
        let nurbs = NurbsCurve::uniform_weights(ctrl(), 3);
        assert!((nurbs.evaluate(0.0) - ctrl()[0]).length() < 1e-5);
        assert!((nurbs.evaluate(1.0) - ctrl()[3]).length() < 1e-5);
    }

    #[test]
    fn higher_weight_pulls_curve() {
        let base = NurbsCurve::uniform_weights(ctrl(), 3);
        let mut weights = vec![1.0; 4];
        weights[1] = 8.0; // pull toward control point 1
        let pulled = NurbsCurve::new(ctrl(), weights, 3);
        // Near the influence of control point 1, the weighted curve should be
        // closer to that control point than the unweighted one.
        let cp = ctrl()[1];
        let d_base = base.evaluate(0.3).distance(cp);
        let d_pull = pulled.evaluate(0.3).distance(cp);
        assert!(d_pull < d_base, "{d_pull} !< {d_base}");
    }

    #[test]
    fn exact_quarter_circle() {
        // Rational quadratic for a 90-degree arc of the unit circle:
        // control (1,0),(1,1),(0,1) with middle weight cos(45) = 1/sqrt(2).
        let w = std::f32::consts::FRAC_1_SQRT_2;
        let arc = NurbsCurve::new(
            vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![1.0, w, 1.0],
            2,
        );
        // Every sampled point must lie on the unit circle (radius 1).
        for i in 0..=10 {
            let p = arc.evaluate(i as f32 / 10.0);
            assert!((p.length() - 1.0).abs() < 1e-3, "radius {} at {i}", p.length());
        }
    }
}
