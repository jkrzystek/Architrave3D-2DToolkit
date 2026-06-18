//! The [`Brush`]: a radius, a strength, and a [`Falloff`], plus the logic to
//! turn a dragged path into evenly spaced dabs and to weight arbitrary target
//! points against a stroke.
//!
//! This is deliberately geometry-agnostic — it produces *weights* at points.
//! Whoever owns the data (a mesh's vertices, a texture's texels, a heightfield's
//! cells, a particle set) multiplies those weights into whatever they edit, so
//! sculpting, painting, and terrain all reuse the same engine.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::falloff::Falloff;

/// A brush definition. `radius` and `strength` are in world / value units;
/// `spacing` is the gap between dabs along a stroke as a fraction of the radius.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Brush {
    pub radius: f32,
    pub strength: f32,
    pub falloff: Falloff,
    /// Dab spacing as a fraction of `radius` (e.g. `0.25` = 4 dabs per radius).
    pub spacing: f32,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            radius: 1.0,
            strength: 1.0,
            falloff: Falloff::Smooth,
            spacing: 0.25,
        }
    }
}

impl Brush {
    /// A brush with the given radius and strength, smooth falloff, default spacing.
    pub fn new(radius: f32, strength: f32) -> Self {
        Self {
            radius,
            strength,
            ..Self::default()
        }
    }

    /// Scale radius and strength by a pressure value (e.g. stylus pressure in
    /// `[0, 1]`), returning a new brush. Other settings are unchanged.
    pub fn with_pressure(self, pressure: f32) -> Self {
        let p = pressure.max(0.0);
        Self {
            radius: self.radius * p,
            strength: self.strength * p,
            ..self
        }
    }

    /// Weight contributed by a single dab centered at `center` to a point at
    /// world distance `distance` from it. `0` at/outside the radius.
    pub fn weight_at_distance(&self, distance: f32) -> f32 {
        if self.radius <= 0.0 || distance >= self.radius {
            return 0.0;
        }
        self.strength * self.falloff.weight(distance / self.radius)
    }

    /// Weight a single dab at `center` contributes to `point`.
    pub fn weight_at_point(&self, center: Vec3, point: Vec3) -> f32 {
        self.weight_at_distance(center.distance(point))
    }

    /// Resample a dragged path into evenly spaced dab centers, `spacing * radius`
    /// apart along the polyline. The first path point is always emitted; an empty
    /// or zero-length path yields its points unchanged.
    pub fn dab_centers(&self, path: &[Vec3]) -> Vec<Vec3> {
        if path.len() < 2 {
            return path.to_vec();
        }
        let step = (self.spacing.max(1e-3)) * self.radius.max(1e-6);
        let mut out = vec![path[0]];
        let mut carry = 0.0_f32; // distance accumulated since the last dab
        for seg in path.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let seg_len = a.distance(b);
            if seg_len <= 1e-12 {
                continue;
            }
            let dir = (b - a) / seg_len;
            let mut d = step - carry; // distance into this segment for next dab
            while d <= seg_len {
                out.push(a + dir * d);
                d += step;
            }
            carry = seg_len - (d - step);
        }
        out
    }

    /// Combined weight of a whole stroke (set of dab `centers`) at `point`,
    /// taking the maximum dab weight. Max (not sum) keeps a single stroke from
    /// over-darkening where dabs overlap — the standard paint/sculpt behaviour.
    pub fn stroke_weight(&self, centers: &[Vec3], point: Vec3) -> f32 {
        centers
            .iter()
            .map(|&c| self.weight_at_point(c, point))
            .fold(0.0, f32::max)
    }

    /// Run `f(index, weight)` for every point in `points` within the influence of
    /// any dab in `centers`, with the accumulated stroke weight. Points with zero
    /// weight are skipped, so callers touch only affected elements.
    pub fn apply<F: FnMut(usize, f32)>(&self, centers: &[Vec3], points: &[Vec3], mut f: F) {
        for (i, &p) in points.iter().enumerate() {
            let w = self.stroke_weight(centers, p);
            if w > 0.0 {
                f(i, w);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_falls_to_zero_at_radius() {
        let b = Brush::new(2.0, 1.0);
        assert!((b.weight_at_distance(0.0) - 1.0).abs() < 1e-6);
        assert_eq!(b.weight_at_distance(2.0), 0.0);
        assert_eq!(b.weight_at_distance(5.0), 0.0);
    }

    #[test]
    fn pressure_scales_radius_and_strength() {
        let b = Brush::new(4.0, 1.0).with_pressure(0.5);
        assert_eq!(b.radius, 2.0);
        assert_eq!(b.strength, 0.5);
    }

    #[test]
    fn dab_centers_are_evenly_spaced() {
        // radius 1, spacing 0.5 -> step 0.5; a length-2 line -> dabs at 0,0.5,1,1.5,2.
        let mut b = Brush::new(1.0, 1.0);
        b.spacing = 0.5;
        let path = [Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)];
        let dabs = b.dab_centers(&path);
        assert_eq!(dabs.len(), 5);
        for (i, d) in dabs.iter().enumerate() {
            assert!((d.x - i as f32 * 0.5).abs() < 1e-5, "dab {i} at {d:?}");
        }
    }

    #[test]
    fn short_path_passes_through() {
        let b = Brush::default();
        assert_eq!(b.dab_centers(&[Vec3::ONE]), vec![Vec3::ONE]);
        assert!(b.dab_centers(&[]).is_empty());
    }

    #[test]
    fn stroke_weight_takes_max() {
        let b = Brush::new(2.0, 1.0);
        // Two overlapping dabs; point sits at the first center.
        let centers = [Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)];
        let w = b.stroke_weight(&centers, Vec3::ZERO);
        assert!((w - 1.0).abs() < 1e-6); // max is the dab centered exactly here
    }

    #[test]
    fn apply_skips_unaffected_points() {
        let b = Brush::new(1.0, 1.0);
        let centers = [Vec3::ZERO];
        let points = [Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)];
        let mut hits = Vec::new();
        b.apply(&centers, &points, |i, w| hits.push((i, w)));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 0);
    }

    #[test]
    fn serde_roundtrip() {
        let b = Brush::new(3.0, 0.8);
        let json = serde_json::to_string(&b).unwrap();
        let back: Brush = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }
}
