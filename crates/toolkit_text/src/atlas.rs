//! A glyph atlas: per-glyph metrics plus a packed SDF texture.
//!
//! The atlas is rasterizer-agnostic — you supply each glyph's SDF image (e.g.
//! from [`crate::coverage_to_sdf`]) and metrics; the builder shelf-packs them
//! into one [`Image`] and records each glyph's UV rectangle.

use std::collections::HashMap;

use glam::Vec2;
use serde::{Deserialize, Serialize};
use toolkit_image::Image;

/// Metrics and atlas placement for one glyph, in arbitrary text units (scale by
/// font size at layout time).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Glyph {
    /// Horizontal pen advance after drawing this glyph.
    pub advance: f32,
    /// Offset from the pen position (baseline) to the glyph quad's top-left.
    pub offset: Vec2,
    /// Size of the glyph quad.
    pub size: Vec2,
    /// Atlas UV of the quad's top-left, in `[0, 1]`.
    pub uv_min: Vec2,
    /// Atlas UV of the quad's bottom-right, in `[0, 1]`.
    pub uv_max: Vec2,
}

/// A finished atlas: the packed SDF image plus a glyph table and vertical
/// metrics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontAtlas {
    pub image: Image,
    glyphs: HashMap<char, Glyph>,
    /// Baseline-to-baseline distance.
    pub line_height: f32,
    /// Baseline to the top of the tallest glyph.
    pub ascent: f32,
}

impl FontAtlas {
    pub fn glyph(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.get(&ch)
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }
}

/// Pending glyph (image + metrics) awaiting packing.
struct PendingGlyph {
    ch: char,
    image: Image,
    advance: f32,
    offset: Vec2,
    size: Vec2,
}

/// Shelf-packs glyph SDF images into a single atlas.
pub struct AtlasBuilder {
    atlas_width: u32,
    padding: u32,
    line_height: f32,
    ascent: f32,
    pending: Vec<PendingGlyph>,
}

impl AtlasBuilder {
    /// Start an atlas of fixed width (height grows to fit). `padding` pixels are
    /// left between glyphs to avoid filtering bleed.
    pub fn new(atlas_width: u32, padding: u32) -> Self {
        Self {
            atlas_width: atlas_width.max(1),
            padding,
            line_height: 0.0,
            ascent: 0.0,
            pending: Vec::new(),
        }
    }

    pub fn set_vertical_metrics(&mut self, line_height: f32, ascent: f32) -> &mut Self {
        self.line_height = line_height;
        self.ascent = ascent;
        self
    }

    /// Queue a glyph: its SDF `image`, pen `advance`, and quad `offset`/`size`.
    pub fn add_glyph(
        &mut self,
        ch: char,
        image: Image,
        advance: f32,
        offset: Vec2,
        size: Vec2,
    ) -> &mut Self {
        self.pending.push(PendingGlyph {
            ch,
            image,
            advance,
            offset,
            size,
        });
        self
    }

    /// Pack everything into a [`FontAtlas`]. Glyphs are placed left-to-right on
    /// shelves; a new shelf starts when the row is full.
    pub fn build(self) -> FontAtlas {
        let pad = self.padding;
        // First pass: lay out rectangles to discover the atlas height.
        let mut pen_x = pad;
        let mut pen_y = pad;
        let mut shelf_height = 0u32;
        let mut placements: Vec<(u32, u32)> = Vec::with_capacity(self.pending.len());

        for g in &self.pending {
            let (gw, gh) = (g.image.width(), g.image.height());
            if pen_x + gw + pad > self.atlas_width {
                // Next shelf.
                pen_x = pad;
                pen_y += shelf_height + pad;
                shelf_height = 0;
            }
            placements.push((pen_x, pen_y));
            pen_x += gw + pad;
            shelf_height = shelf_height.max(gh);
        }
        let atlas_height = (pen_y + shelf_height + pad).max(1);

        // Second pass: blit glyphs and record UVs.
        let mut image = Image::new(self.atlas_width, atlas_height);
        let mut glyphs = HashMap::new();
        let (aw, ah) = (self.atlas_width as f32, atlas_height as f32);

        for (g, (px, py)) in self.pending.into_iter().zip(placements) {
            image.blit(&g.image, px as i32, py as i32);
            let uv_min = Vec2::new(px as f32 / aw, py as f32 / ah);
            let uv_max = Vec2::new(
                (px + g.image.width()) as f32 / aw,
                (py + g.image.height()) as f32 / ah,
            );
            glyphs.insert(
                g.ch,
                Glyph {
                    advance: g.advance,
                    offset: g.offset,
                    size: g.size,
                    uv_min,
                    uv_max,
                },
            );
        }

        FontAtlas {
            image,
            glyphs,
            line_height: self.line_height,
            ascent: self.ascent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32) -> Image {
        Image::from_rgba8(w, h, vec![255u8; (w * h * 4) as usize]).unwrap()
    }

    #[test]
    fn packs_glyphs_and_assigns_uvs() {
        let mut b = AtlasBuilder::new(64, 1);
        b.set_vertical_metrics(12.0, 10.0);
        b.add_glyph('A', solid(8, 10), 9.0, Vec2::ZERO, Vec2::new(8.0, 10.0));
        b.add_glyph('B', solid(8, 10), 9.0, Vec2::ZERO, Vec2::new(8.0, 10.0));
        let atlas = b.build();

        assert_eq!(atlas.glyph_count(), 2);
        let a = atlas.glyph('A').unwrap();
        assert!(a.uv_min.x >= 0.0 && a.uv_max.x <= 1.0);
        assert!(a.advance == 9.0);
        assert_eq!(atlas.line_height, 12.0);
    }

    #[test]
    fn wraps_to_new_shelf_when_full() {
        let mut b = AtlasBuilder::new(20, 1);
        // Three 8-wide glyphs cannot fit on one 20-wide shelf.
        for ch in ['A', 'B', 'C'] {
            b.add_glyph(ch, solid(8, 10), 9.0, Vec2::ZERO, Vec2::new(8.0, 10.0));
        }
        let atlas = b.build();
        // Atlas must be tall enough for at least two shelves.
        assert!(atlas.image.height() > 12);
    }

    #[test]
    fn serde_roundtrip() {
        let mut b = AtlasBuilder::new(64, 1);
        b.add_glyph('X', solid(8, 8), 9.0, Vec2::ZERO, Vec2::new(8.0, 8.0));
        let atlas = b.build();
        let json = serde_json::to_string(&atlas).unwrap();
        let back: FontAtlas = serde_json::from_str(&json).unwrap();
        assert!(back.glyph('X').is_some());
    }
}
