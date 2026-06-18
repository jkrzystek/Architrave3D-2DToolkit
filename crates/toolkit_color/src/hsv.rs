//! HSV (hue/saturation/value), a gamma-space colour model convenient for
//! pickers and hue rotation. Conversions route through sRGB, since HSV is
//! conventionally defined on gamma-encoded values.

use serde::{Deserialize, Serialize};
use toolkit_core::LinearRgba;

/// A colour in HSV: `hue` in degrees `[0, 360)`, `saturation` and `value` in
/// `[0, 1]`. Alpha is carried through unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hsv {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub alpha: f32,
}

impl Hsv {
    pub fn new(hue: f32, saturation: f32, value: f32, alpha: f32) -> Self {
        Self {
            hue,
            saturation,
            value,
            alpha,
        }
    }

    /// Convert from a linear-RGB colour (via sRGB).
    pub fn from_linear(c: LinearRgba) -> Self {
        let [r, g, b, a] = c.to_srgb();
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let hue = if delta <= f32::EPSILON {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        let saturation = if max <= f32::EPSILON { 0.0 } else { delta / max };
        Self::new(hue.rem_euclid(360.0), saturation, max, a)
    }

    /// Convert to a linear-RGB colour (via sRGB).
    pub fn to_linear(self) -> LinearRgba {
        let h = self.hue.rem_euclid(360.0) / 60.0;
        let c = self.value * self.saturation;
        let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
        let m = self.value - c;

        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        LinearRgba::from_srgb(r + m, g + m, b + m, self.alpha)
    }

    /// Return a copy with the hue rotated by `degrees`.
    pub fn rotate_hue(self, degrees: f32) -> Self {
        Self {
            hue: (self.hue + degrees).rem_euclid(360.0),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: LinearRgba, b: LinearRgba, eps: f32) -> bool {
        (a.r - b.r).abs() < eps
            && (a.g - b.g).abs() < eps
            && (a.b - b.b).abs() < eps
            && (a.a - b.a).abs() < eps
    }

    #[test]
    fn primary_red_round_trips() {
        let red = LinearRgba::from_srgb(1.0, 0.0, 0.0, 1.0);
        let hsv = Hsv::from_linear(red);
        assert!((hsv.hue).abs() < 1e-3);
        assert!((hsv.saturation - 1.0).abs() < 1e-3);
        assert!(approx(hsv.to_linear(), red, 1e-4));
    }

    #[test]
    fn green_hue_is_120() {
        let green = LinearRgba::from_srgb(0.0, 1.0, 0.0, 1.0);
        let hsv = Hsv::from_linear(green);
        assert!((hsv.hue - 120.0).abs() < 1e-2);
    }

    #[test]
    fn grey_has_zero_saturation() {
        let grey = LinearRgba::from_srgb(0.5, 0.5, 0.5, 1.0);
        let hsv = Hsv::from_linear(grey);
        assert!(hsv.saturation < 1e-4);
    }

    #[test]
    fn round_trips_arbitrary_colors() {
        for c in [
            LinearRgba::from_srgb(0.2, 0.7, 0.4, 1.0),
            LinearRgba::from_srgb(0.9, 0.1, 0.6, 0.5),
            LinearRgba::from_srgb(0.05, 0.05, 0.8, 1.0),
        ] {
            assert!(approx(Hsv::from_linear(c).to_linear(), c, 1e-4));
        }
    }

    #[test]
    fn rotate_hue_wraps() {
        let h = Hsv::new(350.0, 1.0, 1.0, 1.0).rotate_hue(20.0);
        assert!((h.hue - 10.0).abs() < 1e-4);
    }

    #[test]
    fn serde_roundtrip() {
        let h = Hsv::new(120.0, 0.5, 0.8, 1.0);
        let json = serde_json::to_string(&h).unwrap();
        let back: Hsv = serde_json::from_str(&json).unwrap();
        assert_eq!(h, back);
    }
}
