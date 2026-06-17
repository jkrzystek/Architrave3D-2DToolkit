use toolkit_core::BlendMode;

/// Blend two premultiplied linear RGBA colors.
///
/// `base` is the existing color, `top` is being painted over it.
/// `mode` selects the per-channel blend formula, and `opacity` (0..=1)
/// controls how much of the blended result is mixed in.
///
/// Both inputs and output are premultiplied linear RGBA `[R, G, B, A]`.
pub fn blend(base: [f32; 4], top: [f32; 4], mode: BlendMode, opacity: f32) -> [f32; 4] {
    let opacity = opacity.clamp(0.0, 1.0);
    let top_a = top[3] * opacity;

    if top_a <= 0.0 {
        return base;
    }

    let base_a = base[3];

    // Un-premultiply for the per-channel blend formulas.
    let (base_rgb, top_rgb) = unpremultiply(base, top);

    // Apply the selected blend formula per channel.
    let blended_rgb = [
        blend_channel(base_rgb[0], top_rgb[0], mode),
        blend_channel(base_rgb[1], top_rgb[1], mode),
        blend_channel(base_rgb[2], top_rgb[2], mode),
    ];

    // Porter-Duff src-over compositing.
    let out_a = top_a + base_a * (1.0 - top_a);
    if out_a <= 0.0 {
        return [0.0; 4];
    }

    let mut out = [0.0_f32; 4];
    for i in 0..3 {
        // Composite the blended color with the base via src-over.
        out[i] = (top_a * blended_rgb[i] + base_a * base_rgb[i] * (1.0 - top_a)) / out_a;
        // Re-premultiply.
        out[i] *= out_a;
    }
    out[3] = out_a;
    out
}

/// Un-premultiply RGB channels. Returns (base_rgb, top_rgb) with values in 0..1.
fn unpremultiply(base: [f32; 4], top: [f32; 4]) -> ([f32; 3], [f32; 3]) {
    let unp = |c: [f32; 4]| -> [f32; 3] {
        if c[3] > 0.0 {
            [c[0] / c[3], c[1] / c[3], c[2] / c[3]]
        } else {
            [0.0; 3]
        }
    };
    (unp(base), unp(top))
}

/// Apply a blend formula to a single channel (straight, un-premultiplied values).
fn blend_channel(base: f32, top: f32, mode: BlendMode) -> f32 {
    match mode {
        BlendMode::Normal | BlendMode::PassThrough => top,

        BlendMode::Multiply => base * top,

        BlendMode::Screen => 1.0 - (1.0 - base) * (1.0 - top),

        BlendMode::Overlay => {
            if base < 0.5 {
                2.0 * base * top
            } else {
                1.0 - 2.0 * (1.0 - base) * (1.0 - top)
            }
        }

        BlendMode::SoftLight => {
            // W3C formula
            if top <= 0.5 {
                base - (1.0 - 2.0 * top) * base * (1.0 - base)
            } else {
                let d = if base <= 0.25 {
                    ((16.0 * base - 12.0) * base + 4.0) * base
                } else {
                    base.sqrt()
                };
                base + (2.0 * top - 1.0) * (d - base)
            }
        }

        BlendMode::HardLight => {
            // Same as Overlay but with base/top swapped.
            if top < 0.5 {
                2.0 * base * top
            } else {
                1.0 - 2.0 * (1.0 - base) * (1.0 - top)
            }
        }

        BlendMode::ColorDodge => {
            if base <= 0.0 {
                0.0
            } else if top >= 1.0 {
                1.0
            } else {
                (base / (1.0 - top)).min(1.0)
            }
        }

        BlendMode::ColorBurn => {
            if base >= 1.0 {
                1.0
            } else if top <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - base) / top).min(1.0)
            }
        }

        BlendMode::Darken => base.min(top),

        BlendMode::Lighten => base.max(top),

        BlendMode::Difference => (base - top).abs(),

        BlendMode::Exclusion => base + top - 2.0 * base * top,

        BlendMode::Add => (base + top).min(1.0),

        BlendMode::Subtract => (base - top).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    fn assert_rgba_eq(actual: [f32; 4], expected: [f32; 4], msg: &str) {
        for i in 0..4 {
            assert!(
                (actual[i] - expected[i]).abs() < EPSILON,
                "{msg}: channel {i}: expected {}, got {}",
                expected[i],
                actual[i],
            );
        }
    }

    // ---- identity tests ----

    #[test]
    fn blend_with_transparent_top_returns_base() {
        let base = [0.5, 0.3, 0.1, 0.8];
        let transparent = [0.0, 0.0, 0.0, 0.0];
        let result = blend(base, transparent, BlendMode::Normal, 1.0);
        assert_rgba_eq(result, base, "transparent top");
    }

    #[test]
    fn blend_with_zero_opacity_returns_base() {
        let base = [0.5, 0.3, 0.1, 0.8];
        let top = [1.0, 1.0, 1.0, 1.0];
        let result = blend(base, top, BlendMode::Normal, 0.0);
        assert_rgba_eq(result, base, "zero opacity");
    }

    // ---- Normal mode ----

    #[test]
    fn normal_opaque_over_opaque() {
        // Fully opaque top over fully opaque base in Normal mode
        // should yield the top color.
        let base = [0.2, 0.4, 0.6, 1.0];
        let top = [0.8, 0.6, 0.4, 1.0];
        let result = blend(base, top, BlendMode::Normal, 1.0);
        assert_rgba_eq(result, top, "normal opaque/opaque");
    }

    #[test]
    fn normal_half_opacity() {
        // Opaque white over opaque black at 50% opacity -> mid gray
        let base = [0.0, 0.0, 0.0, 1.0]; // premul black
        let top = [1.0, 1.0, 1.0, 1.0]; // premul white
        let result = blend(base, top, BlendMode::Normal, 0.5);
        // out_a = 0.5 + 1.0*(1-0.5) = 1.0
        // out_r = (0.5 * 1.0 + 1.0 * 0.0 * 0.5) / 1.0 * 1.0 = 0.5
        assert_rgba_eq(result, [0.5, 0.5, 0.5, 1.0], "normal half opacity");
    }

    // ---- Multiply ----

    #[test]
    fn multiply_known_values() {
        // Premultiplied: both opaque
        let base = [0.5, 0.5, 0.5, 1.0];
        let top = [0.4, 0.6, 0.8, 1.0];
        let result = blend(base, top, BlendMode::Multiply, 1.0);
        // multiply(0.5, 0.4) = 0.2; multiply(0.5, 0.6) = 0.3; multiply(0.5, 0.8) = 0.4
        // out_a = 1.0, and with full coverage the premul result = straight result
        assert_rgba_eq(result, [0.2, 0.3, 0.4, 1.0], "multiply");
    }

    // ---- Screen ----

    #[test]
    fn screen_known_values() {
        let base = [0.5, 0.5, 0.5, 1.0];
        let top = [0.5, 0.5, 0.5, 1.0];
        let result = blend(base, top, BlendMode::Screen, 1.0);
        // screen(0.5, 0.5) = 1 - 0.5*0.5 = 0.75
        assert_rgba_eq(result, [0.75, 0.75, 0.75, 1.0], "screen");
    }

    #[test]
    fn screen_white_yields_white() {
        let base = [0.3, 0.3, 0.3, 1.0];
        let top = [1.0, 1.0, 1.0, 1.0];
        let result = blend(base, top, BlendMode::Screen, 1.0);
        assert_rgba_eq(result, [1.0, 1.0, 1.0, 1.0], "screen white");
    }

    // ---- Overlay ----

    #[test]
    fn overlay_known_values() {
        // overlay(base, top): base < 0.5 => 2*base*top; base >= 0.5 => 1-2*(1-base)*(1-top)
        let base = [0.25, 0.75, 0.5, 1.0]; // channels: dark, light, mid
        let top = [0.5, 0.5, 0.5, 1.0];
        let result = blend(base, top, BlendMode::Overlay, 1.0);
        // ch0: base=0.25 < 0.5 => 2*0.25*0.5 = 0.25
        // ch1: base=0.75 >= 0.5 => 1-2*0.25*0.5 = 0.75
        // ch2: base=0.5 >= 0.5 => 1-2*0.5*0.5 = 0.5
        assert_rgba_eq(result, [0.25, 0.75, 0.5, 1.0], "overlay");
    }

    // ---- Difference ----

    #[test]
    fn difference_same_color_is_black() {
        let c = [0.6, 0.6, 0.6, 1.0];
        let result = blend(c, c, BlendMode::Difference, 1.0);
        assert_rgba_eq(result, [0.0, 0.0, 0.0, 1.0], "difference same color");
    }

    // ---- Add ----

    #[test]
    fn add_clamps_to_one() {
        let base = [0.7, 0.7, 0.7, 1.0];
        let top = [0.5, 0.5, 0.5, 1.0];
        let result = blend(base, top, BlendMode::Add, 1.0);
        assert_rgba_eq(result, [1.0, 1.0, 1.0, 1.0], "add clamp");
    }

    // ---- Subtract ----

    #[test]
    fn subtract_clamps_to_zero() {
        let base = [0.3, 0.3, 0.3, 1.0];
        let top = [0.5, 0.5, 0.5, 1.0];
        let result = blend(base, top, BlendMode::Subtract, 1.0);
        assert_rgba_eq(result, [0.0, 0.0, 0.0, 1.0], "subtract clamp");
    }

    // ---- Darken / Lighten ----

    #[test]
    fn darken_picks_darker() {
        let base = [0.3, 0.7, 0.5, 1.0];
        let top = [0.6, 0.4, 0.5, 1.0];
        let result = blend(base, top, BlendMode::Darken, 1.0);
        assert_rgba_eq(result, [0.3, 0.4, 0.5, 1.0], "darken");
    }

    #[test]
    fn lighten_picks_lighter() {
        let base = [0.3, 0.7, 0.5, 1.0];
        let top = [0.6, 0.4, 0.5, 1.0];
        let result = blend(base, top, BlendMode::Lighten, 1.0);
        assert_rgba_eq(result, [0.6, 0.7, 0.5, 1.0], "lighten");
    }

    // ---- Exclusion ----

    #[test]
    fn exclusion_with_black_returns_base() {
        let base = [0.6, 0.6, 0.6, 1.0];
        let top = [0.0, 0.0, 0.0, 1.0];
        let result = blend(base, top, BlendMode::Exclusion, 1.0);
        assert_rgba_eq(result, [0.6, 0.6, 0.6, 1.0], "exclusion black");
    }

    // ---- Alpha compositing ----

    #[test]
    fn semi_transparent_compositing() {
        // Base: 50% alpha red (premul: [0.5, 0, 0, 0.5])
        // Top: 50% alpha green (premul: [0, 0.25, 0, 0.5])
        let base = [0.5, 0.0, 0.0, 0.5];
        let top = [0.0, 0.25, 0.0, 0.5];
        let result = blend(base, top, BlendMode::Normal, 1.0);
        // out_a = 0.5 + 0.5*(1-0.5) = 0.75
        // un-premul top green = 0.25/0.5 = 0.5
        // un-premul base red = 0.5/0.5 = 1.0
        // Normal: top color = [0.0, 0.5, 0.0]
        // out_r = (0.5 * 0.0 + 0.5 * 1.0 * 0.5) / 0.75 = 0.25/0.75 = 0.3333
        // premul: 0.3333 * 0.75 = 0.25
        // out_g = (0.5 * 0.5 + 0.5 * 0.0 * 0.5) / 0.75 = 0.25/0.75 = 0.3333
        // premul: 0.3333 * 0.75 = 0.25
        assert_rgba_eq(result, [0.25, 0.25, 0.0, 0.75], "semi-transparent");
    }
}
