//! OKLab / OKLch — a perceptually uniform colour space. Mixing and gradient
//! interpolation here avoids the muddy mid-tones you get blending in sRGB.
//!
//! The matrices below are Björn Ottosson's published linear-sRGB ↔ OKLab
//! transform (<https://bottosson.github.io/posts/oklab/>).

use serde::{Deserialize, Serialize};
use toolkit_core::LinearRgba;

/// A colour in OKLab: `l` lightness `[0, 1]`, `a`/`b` opponent axes
/// (green–red, blue–yellow), roughly `[-0.4, 0.4]`. Alpha carried through.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Oklab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
    pub alpha: f32,
}

impl Oklab {
    pub fn new(l: f32, a: f32, b: f32, alpha: f32) -> Self {
        Self { l, a, b, alpha }
    }

    pub fn from_linear(c: LinearRgba) -> Self {
        let l = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
        let m = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
        let s = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;

        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();

        Self {
            l: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
            alpha: c.a,
        }
    }

    pub fn to_linear(self) -> LinearRgba {
        let l_ = self.l + 0.3963377774 * self.a + 0.2158037573 * self.b;
        let m_ = self.l - 0.1055613458 * self.a - 0.0638541728 * self.b;
        let s_ = self.l - 0.0894841775 * self.a - 1.2914855480 * self.b;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        LinearRgba {
            r: 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
            g: -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
            b: -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
            a: self.alpha,
        }
    }

    /// Linear interpolation in OKLab space.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            l: self.l + (other.l - self.l) * t,
            a: self.a + (other.a - self.a) * t,
            b: self.b + (other.b - self.b) * t,
            alpha: self.alpha + (other.alpha - self.alpha) * t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: LinearRgba, b: LinearRgba, eps: f32) -> bool {
        (a.r - b.r).abs() < eps && (a.g - b.g).abs() < eps && (a.b - b.b).abs() < eps
    }

    #[test]
    fn white_has_lightness_one() {
        let lab = Oklab::from_linear(LinearRgba::WHITE);
        assert!((lab.l - 1.0).abs() < 1e-3);
        assert!(lab.a.abs() < 1e-3 && lab.b.abs() < 1e-3);
    }

    #[test]
    fn black_has_zero_lightness() {
        let lab = Oklab::from_linear(LinearRgba::BLACK);
        assert!(lab.l.abs() < 1e-3);
    }

    #[test]
    fn round_trips() {
        for c in [
            LinearRgba::from_srgb(0.2, 0.7, 0.4, 1.0),
            LinearRgba::from_srgb(0.9, 0.1, 0.6, 1.0),
            LinearRgba::from_srgb(0.5, 0.5, 0.5, 1.0),
        ] {
            assert!(approx(Oklab::from_linear(c).to_linear(), c, 1e-4));
        }
    }

    #[test]
    fn midpoint_lerp_is_between() {
        let a = Oklab::from_linear(LinearRgba::BLACK);
        let b = Oklab::from_linear(LinearRgba::WHITE);
        let mid = a.lerp(b, 0.5);
        assert!((mid.l - 0.5).abs() < 1e-4);
    }

    #[test]
    fn serde_roundtrip() {
        let lab = Oklab::new(0.5, 0.1, -0.1, 1.0);
        let json = serde_json::to_string(&lab).unwrap();
        let back: Oklab = serde_json::from_str(&json).unwrap();
        assert_eq!(lab, back);
    }
}
