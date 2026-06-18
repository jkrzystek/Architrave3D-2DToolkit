//! Brush falloff profiles: how influence drops from the center (`t = 0`) to the
//! edge (`t = 1`) of a brush.
//!
//! Every profile returns `1` at the center and `0` at (or beyond) the edge, with
//! a smooth or sharp shape in between. These are the same curves sculpt/paint
//! tools expose as "brush hardness".

use serde::{Deserialize, Serialize};

/// A radial falloff shape, evaluated on a normalized distance `t ∈ [0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Falloff {
    /// Full strength everywhere inside the radius, then a hard cutoff.
    Constant,
    /// Straight ramp `1 - t`.
    Linear,
    /// Smoothstep (zero slope at both ends) — the default "soft" brush.
    Smooth,
    /// `(1 - t)²` — concentrated near the center.
    Sharp,
    /// `sqrt(1 - t²)` — a spherical dome.
    Sphere,
}

impl Default for Falloff {
    fn default() -> Self {
        Falloff::Smooth
    }
}

impl Falloff {
    /// Weight in `[0, 1]` for a normalized distance `t`. Values of `t` outside
    /// `[0, 1]` clamp to the endpoints (`1` at/inside the center, `0` past edge).
    pub fn weight(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Falloff::Constant => 1.0,
            Falloff::Linear => 1.0 - t,
            // 1 at t=0, 0 at t=1, flat at both ends.
            Falloff::Smooth => 1.0 - (3.0 * t * t - 2.0 * t * t * t),
            Falloff::Sharp => (1.0 - t) * (1.0 - t),
            Falloff::Sphere => (1.0 - t * t).max(0.0).sqrt(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_one_and_zero() {
        for f in [
            Falloff::Constant,
            Falloff::Linear,
            Falloff::Smooth,
            Falloff::Sharp,
            Falloff::Sphere,
        ] {
            assert!((f.weight(0.0) - 1.0).abs() < 1e-6, "{f:?} center");
            // Constant stays 1 right up to the edge; others reach 0.
            if f != Falloff::Constant {
                assert!(f.weight(1.0).abs() < 1e-6, "{f:?} edge");
            }
        }
    }

    #[test]
    fn clamps_outside_range() {
        assert_eq!(Falloff::Linear.weight(-1.0), 1.0);
        assert_eq!(Falloff::Linear.weight(2.0), 0.0);
    }

    #[test]
    fn smooth_is_monotonic_decreasing() {
        let mut prev = 1.1;
        for i in 0..=10 {
            let w = Falloff::Smooth.weight(i as f32 / 10.0);
            assert!(w <= prev + 1e-6, "not decreasing at {i}");
            prev = w;
        }
    }

    #[test]
    fn serde_roundtrip() {
        let json = serde_json::to_string(&Falloff::Sphere).unwrap();
        let back: Falloff = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Falloff::Sphere);
    }
}
