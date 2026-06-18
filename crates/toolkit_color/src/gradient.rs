//! Multi-stop colour gradients, sampled in either linear-RGB or OKLab space.

use serde::{Deserialize, Serialize};
use toolkit_core::LinearRgba;

use crate::oklab::Oklab;

/// Which colour space a gradient interpolates in. OKLab gives perceptually
/// even transitions; linear RGB matches what GPU blending does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InterpolationSpace {
    LinearRgb,
    #[default]
    Oklab,
}

/// A colour at a normalised position along a gradient.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorStop {
    /// Position in `[0, 1]`.
    pub position: f32,
    pub color: LinearRgba,
}

/// A gradient defined by colour stops, kept sorted by position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    stops: Vec<ColorStop>,
    pub space: InterpolationSpace,
}

impl Gradient {
    pub fn new(space: InterpolationSpace) -> Self {
        Self {
            stops: Vec::new(),
            space,
        }
    }

    /// A two-stop gradient from `start` (t=0) to `end` (t=1).
    pub fn two_stop(start: LinearRgba, end: LinearRgba, space: InterpolationSpace) -> Self {
        let mut g = Self::new(space);
        g.add_stop(0.0, start);
        g.add_stop(1.0, end);
        g
    }

    /// Insert a stop, keeping stops sorted by position.
    pub fn add_stop(&mut self, position: f32, color: LinearRgba) -> &mut Self {
        let stop = ColorStop { position, color };
        let idx = self
            .stops
            .binary_search_by(|s| s.position.total_cmp(&position))
            .unwrap_or_else(|e| e);
        self.stops.insert(idx, stop);
        self
    }

    pub fn stops(&self) -> &[ColorStop] {
        &self.stops
    }

    /// Sample the gradient at `t`. Clamps to the end stops outside `[first,
    /// last]`. Returns black if the gradient has no stops.
    pub fn sample(&self, t: f32) -> LinearRgba {
        match self.stops.len() {
            0 => LinearRgba::BLACK,
            1 => self.stops[0].color,
            _ => {
                let first = &self.stops[0];
                let last = &self.stops[self.stops.len() - 1];
                if t <= first.position {
                    return first.color;
                }
                if t >= last.position {
                    return last.color;
                }
                // Find the bracketing segment.
                let i = self
                    .stops
                    .partition_point(|s| s.position <= t)
                    .saturating_sub(1);
                let a = &self.stops[i];
                let b = &self.stops[i + 1];
                let span = (b.position - a.position).max(1e-9);
                let local = (t - a.position) / span;
                self.mix(a.color, b.color, local)
            }
        }
    }

    /// Produce `count` evenly spaced samples across `[0, 1]`.
    pub fn ramp(&self, count: usize) -> Vec<LinearRgba> {
        if count == 0 {
            return Vec::new();
        }
        if count == 1 {
            return vec![self.sample(0.5)];
        }
        (0..count)
            .map(|i| self.sample(i as f32 / (count - 1) as f32))
            .collect()
    }

    fn mix(&self, a: LinearRgba, b: LinearRgba, t: f32) -> LinearRgba {
        match self.space {
            InterpolationSpace::LinearRgb => a.lerp(b, t),
            InterpolationSpace::Oklab => {
                Oklab::from_linear(a).lerp(Oklab::from_linear(b), t).to_linear()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_endpoints() {
        let g = Gradient::two_stop(LinearRgba::BLACK, LinearRgba::WHITE, InterpolationSpace::LinearRgb);
        assert_eq!(g.sample(0.0), LinearRgba::BLACK);
        assert_eq!(g.sample(1.0), LinearRgba::WHITE);
    }

    #[test]
    fn clamps_outside_range() {
        let g = Gradient::two_stop(LinearRgba::BLACK, LinearRgba::WHITE, InterpolationSpace::LinearRgb);
        assert_eq!(g.sample(-5.0), LinearRgba::BLACK);
        assert_eq!(g.sample(5.0), LinearRgba::WHITE);
    }

    #[test]
    fn linear_midpoint() {
        let g = Gradient::two_stop(LinearRgba::BLACK, LinearRgba::WHITE, InterpolationSpace::LinearRgb);
        let mid = g.sample(0.5);
        assert!((mid.r - 0.5).abs() < 1e-5);
    }

    #[test]
    fn three_stops_pick_correct_segment() {
        let mut g = Gradient::new(InterpolationSpace::LinearRgb);
        g.add_stop(0.0, LinearRgba::new(0.0, 0.0, 0.0, 1.0));
        g.add_stop(0.5, LinearRgba::new(1.0, 0.0, 0.0, 1.0));
        g.add_stop(1.0, LinearRgba::new(1.0, 1.0, 1.0, 1.0));
        // Quarter way: halfway between black and red.
        let c = g.sample(0.25);
        assert!((c.r - 0.5).abs() < 1e-5 && c.g.abs() < 1e-5);
        // Three-quarters: halfway between red and white.
        let c = g.sample(0.75);
        assert!((c.g - 0.5).abs() < 1e-5);
    }

    #[test]
    fn ramp_has_requested_length_and_endpoints() {
        let g = Gradient::two_stop(LinearRgba::BLACK, LinearRgba::WHITE, InterpolationSpace::Oklab);
        let r = g.ramp(5);
        assert_eq!(r.len(), 5);
        assert_eq!(r[0], LinearRgba::BLACK);
    }

    #[test]
    fn oklab_midpoint_differs_from_linear() {
        // Perceptual mid-grey (L=0.5) lands well below linear 0.5 once decoded
        // back to linear RGB, because OKLab lightness is roughly a cube root.
        let g = Gradient::two_stop(LinearRgba::BLACK, LinearRgba::WHITE, InterpolationSpace::Oklab);
        let mid = g.sample(0.5);
        assert!(mid.r < 0.5 && mid.r > 0.0);
    }

    #[test]
    fn serde_roundtrip() {
        let g = Gradient::two_stop(LinearRgba::BLACK, LinearRgba::WHITE, InterpolationSpace::Oklab);
        let json = serde_json::to_string(&g).unwrap();
        let back: Gradient = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}
