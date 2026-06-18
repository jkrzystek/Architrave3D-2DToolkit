//! Lay out a string against a [`FontAtlas`]: positioned glyph quads with line
//! breaking and optional greedy word wrap.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::atlas::{FontAtlas, Glyph};

/// A glyph placed in layout space. `position` is the quad's top-left; combine
/// with [`Glyph::uv_min`]/[`Glyph::uv_max`] to emit a textured quad.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PositionedGlyph {
    pub ch: char,
    /// Top-left of the glyph quad in layout space (y grows downward).
    pub position: Vec2,
    pub glyph: Glyph,
}

/// Layout configuration.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutOptions {
    /// Scale applied to all atlas metrics (the target font size factor).
    pub scale: f32,
    /// Maximum line width in layout units before wrapping; `None` disables wrap.
    pub max_width: Option<f32>,
    /// Extra spacing added between lines, on top of the atlas line height.
    pub line_gap: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            max_width: None,
            line_gap: 0.0,
        }
    }
}

/// The result of laying out text.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TextLayout {
    pub glyphs: Vec<PositionedGlyph>,
    /// Bounding width of the laid-out text.
    pub width: f32,
    /// Bounding height (line count × scaled line height).
    pub height: f32,
}

/// Lay out `text` using `atlas` and `options`. Unknown characters are skipped
/// (but spaces still advance). Wrapping is greedy on whitespace; explicit `\n`
/// always breaks.
pub fn layout_text(atlas: &FontAtlas, text: &str, options: &LayoutOptions) -> TextLayout {
    let scale = options.scale;
    let line_height = atlas.line_height * scale + options.line_gap;

    let mut glyphs = Vec::new();
    let mut max_width = 0.0f32;
    let mut pen_y = 0.0f32;
    let mut line_count = 0usize;

    for raw_line in text.split('\n') {
        for wrapped in wrap_line(atlas, raw_line, scale, options.max_width) {
            let mut pen_x = 0.0f32;
            for ch in wrapped.chars() {
                if let Some(g) = atlas.glyph(ch) {
                    let position = Vec2::new(
                        pen_x + g.offset.x * scale,
                        pen_y + atlas.ascent * scale - g.offset.y * scale,
                    );
                    glyphs.push(PositionedGlyph {
                        ch,
                        position,
                        glyph: *g,
                    });
                    pen_x += g.advance * scale;
                } else if ch == ' ' {
                    // Space with no glyph: advance by a quarter em-ish fallback
                    // if the atlas lacks a space glyph.
                    pen_x += atlas.line_height * 0.25 * scale;
                }
            }
            max_width = max_width.max(pen_x);
            pen_y += line_height;
            line_count += 1;
        }
    }

    TextLayout {
        glyphs,
        width: max_width,
        height: line_count as f32 * line_height,
    }
}

/// Width of a run of text in layout units (sum of advances).
fn run_width(atlas: &FontAtlas, run: &str, scale: f32) -> f32 {
    run.chars()
        .map(|ch| match atlas.glyph(ch) {
            Some(g) => g.advance * scale,
            None if ch == ' ' => atlas.line_height * 0.25 * scale,
            None => 0.0,
        })
        .sum()
}

/// Greedily break `line` into sub-lines no wider than `max_width`.
fn wrap_line(atlas: &FontAtlas, line: &str, scale: f32, max_width: Option<f32>) -> Vec<String> {
    let Some(limit) = max_width else {
        return vec![line.to_string()];
    };

    let mut out = Vec::new();
    let mut current = String::new();
    for word in line.split(' ') {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if run_width(atlas, &candidate, scale) > limit && !current.is_empty() {
            out.push(std::mem::take(&mut current));
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() || out.is_empty() {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasBuilder;
    use toolkit_image::Image;

    fn glyph_img() -> Image {
        Image::from_rgba8(8, 10, vec![255u8; 8 * 10 * 4]).unwrap()
    }

    /// Monospace test atlas: every letter advances 10 units.
    fn mono_atlas() -> FontAtlas {
        let mut b = AtlasBuilder::new(128, 1);
        b.set_vertical_metrics(12.0, 10.0);
        for ch in 'a'..='z' {
            b.add_glyph(ch, glyph_img(), 10.0, Vec2::ZERO, Vec2::new(8.0, 10.0));
        }
        b.build()
    }

    #[test]
    fn lays_out_each_glyph() {
        let atlas = mono_atlas();
        let layout = layout_text(&atlas, "abc", &LayoutOptions::default());
        assert_eq!(layout.glyphs.len(), 3);
        // Advances accumulate: 0, 10, 20.
        assert!((layout.glyphs[1].position.x - 10.0).abs() < 1e-4);
        assert!((layout.width - 30.0).abs() < 1e-4);
    }

    #[test]
    fn newline_starts_a_second_line() {
        let atlas = mono_atlas();
        let layout = layout_text(&atlas, "ab\ncd", &LayoutOptions::default());
        assert_eq!(layout.glyphs.len(), 4);
        // First glyph of line 2 sits a line-height lower.
        let line2 = layout.glyphs[2].position.y;
        let line1 = layout.glyphs[0].position.y;
        assert!(line2 > line1);
        assert!((layout.height - 24.0).abs() < 1e-4); // two lines × 12
    }

    #[test]
    fn scale_multiplies_advances() {
        let atlas = mono_atlas();
        let opts = LayoutOptions {
            scale: 2.0,
            ..Default::default()
        };
        let layout = layout_text(&atlas, "ab", &opts);
        assert!((layout.glyphs[1].position.x - 20.0).abs() < 1e-4);
    }

    #[test]
    fn word_wrap_breaks_long_lines() {
        let atlas = mono_atlas();
        let opts = LayoutOptions {
            max_width: Some(45.0), // ~4 glyphs of width 10
            ..Default::default()
        };
        // "aaa bbb" is 7 chars = 70 units unwrapped; should wrap to two lines.
        let layout = layout_text(&atlas, "aaa bbb", &opts);
        let max_y = layout.glyphs.iter().map(|g| g.position.y).fold(0.0, f32::max);
        let min_y = layout.glyphs.iter().map(|g| g.position.y).fold(f32::MAX, f32::min);
        assert!(max_y > min_y, "expected wrapping to produce multiple lines");
    }

    #[test]
    fn unknown_chars_are_skipped() {
        let atlas = mono_atlas();
        // Digits aren't in the atlas; only the three letters are placed.
        let layout = layout_text(&atlas, "a1b2c", &LayoutOptions::default());
        assert_eq!(layout.glyphs.len(), 3);
    }

    #[test]
    fn serde_roundtrip() {
        let atlas = mono_atlas();
        let layout = layout_text(&atlas, "abc", &LayoutOptions::default());
        let json = serde_json::to_string(&layout).unwrap();
        let back: TextLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(back.glyphs.len(), 3);
    }
}
