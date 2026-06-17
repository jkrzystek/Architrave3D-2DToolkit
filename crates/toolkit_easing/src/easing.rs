use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// A named easing function. Apply it with [`ease`] (or [`Easing::apply`]); the
/// input `t` is clamped to `[0, 1]` and the output is generally in `[0, 1]`
/// (overshooting variants like `Back`/`Elastic` may exceed it briefly).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    SineIn,
    SineOut,
    SineInOut,
    ExpoIn,
    ExpoOut,
    ExpoInOut,
    BackIn,
    BackOut,
    BackInOut,
    ElasticOut,
    BounceOut,
    /// Hermite smoothstep: `t²(3 - 2t)`.
    SmoothStep,
    /// Perlin smootherstep: `t³(6t² - 15t + 10)`.
    SmootherStep,
}

impl Easing {
    pub fn apply(self, t: f32) -> f32 {
        ease(self, t)
    }
}

/// Evaluate an easing function at `t` (clamped to `[0, 1]`).
pub fn ease(easing: Easing, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    use Easing::*;
    match easing {
        Linear => t,
        QuadIn => t * t,
        QuadOut => 1.0 - (1.0 - t) * (1.0 - t),
        QuadInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        CubicIn => t * t * t,
        CubicOut => 1.0 - (1.0 - t).powi(3),
        CubicInOut => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }
        SineIn => 1.0 - (t * PI / 2.0).cos(),
        SineOut => (t * PI / 2.0).sin(),
        SineInOut => -((PI * t).cos() - 1.0) / 2.0,
        ExpoIn => {
            if t == 0.0 {
                0.0
            } else {
                2f32.powf(10.0 * t - 10.0)
            }
        }
        ExpoOut => {
            if t >= 1.0 {
                1.0
            } else {
                1.0 - 2f32.powf(-10.0 * t)
            }
        }
        ExpoInOut => {
            if t == 0.0 {
                0.0
            } else if t >= 1.0 {
                1.0
            } else if t < 0.5 {
                2f32.powf(20.0 * t - 10.0) / 2.0
            } else {
                (2.0 - 2f32.powf(-20.0 * t + 10.0)) / 2.0
            }
        }
        BackIn => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            c3 * t * t * t - c1 * t * t
        }
        BackOut => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            let u = t - 1.0;
            1.0 + c3 * u * u * u + c1 * u * u
        }
        BackInOut => {
            let c1 = 1.70158;
            let c2 = c1 * 1.525;
            if t < 0.5 {
                ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
            } else {
                ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
            }
        }
        ElasticOut => {
            if t == 0.0 {
                0.0
            } else if t >= 1.0 {
                1.0
            } else {
                let c4 = (2.0 * PI) / 3.0;
                2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
            }
        }
        BounceOut => bounce_out(t),
        SmoothStep => t * t * (3.0 - 2.0 * t),
        SmootherStep => t * t * t * (t * (t * 6.0 - 15.0) + 10.0),
    }
}

fn bounce_out(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

// -- Interpolation helpers ---------------------------------------------------

/// Linear interpolation.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Inverse of [`lerp`]: where does `v` fall between `a` and `b` (as a `0..1`)?
pub fn inverse_lerp(a: f32, b: f32, v: f32) -> f32 {
    if (b - a).abs() < f32::EPSILON {
        0.0
    } else {
        (v - a) / (b - a)
    }
}

/// Remap `v` from one range to another.
pub fn remap(v: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    lerp(out_min, out_max, inverse_lerp(in_min, in_max, v))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoints(e: Easing) {
        assert!(ease(e, 0.0).abs() < 1e-4, "{e:?} f(0) != 0");
        assert!((ease(e, 1.0) - 1.0).abs() < 1e-4, "{e:?} f(1) != 1");
    }

    #[test]
    fn all_easings_pin_endpoints() {
        use Easing::*;
        for e in [
            Linear, QuadIn, QuadOut, QuadInOut, CubicIn, CubicOut, CubicInOut, SineIn, SineOut,
            SineInOut, ExpoIn, ExpoOut, ExpoInOut, BackIn, BackOut, BackInOut, ElasticOut,
            BounceOut, SmoothStep, SmootherStep,
        ] {
            endpoints(e);
        }
    }

    #[test]
    fn input_is_clamped() {
        assert_eq!(ease(Easing::Linear, -5.0), 0.0);
        assert_eq!(ease(Easing::Linear, 5.0), 1.0);
    }

    #[test]
    fn quad_in_is_below_linear_midway() {
        // Ease-in starts slow.
        assert!(ease(Easing::QuadIn, 0.5) < 0.5);
    }

    #[test]
    fn smoothstep_symmetric_midpoint() {
        assert!((ease(Easing::SmoothStep, 0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn lerp_and_inverse() {
        assert!((lerp(10.0, 20.0, 0.25) - 12.5).abs() < 1e-6);
        assert!((inverse_lerp(10.0, 20.0, 12.5) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn remap_ranges() {
        assert!((remap(5.0, 0.0, 10.0, 0.0, 100.0) - 50.0).abs() < 1e-6);
    }
}
