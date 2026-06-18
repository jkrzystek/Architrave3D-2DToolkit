//! Signed-distance-field text: glyph SDF generation, atlas packing, and layout.
//!
//! The crate is rasterizer-agnostic. Supply each glyph's coverage mask from any
//! font source; [`coverage_to_sdf`] converts it to a crisp-at-any-scale SDF,
//! [`AtlasBuilder`] packs glyph SDFs into one [`FontAtlas`], and [`layout_text`]
//! positions a string (with line breaks and greedy word wrap) into textured
//! quads.
//!
//! ```
//! use glam::Vec2;
//! use toolkit_text::{AtlasBuilder, layout_text, LayoutOptions};
//! use toolkit_image::Image;
//!
//! let mut builder = AtlasBuilder::new(64, 1);
//! builder.set_vertical_metrics(12.0, 10.0);
//! let glyph = Image::from_rgba8(8, 10, vec![255; 8 * 10 * 4]).unwrap();
//! builder.add_glyph('A', glyph, 9.0, Vec2::ZERO, Vec2::new(8.0, 10.0));
//! let atlas = builder.build();
//!
//! let layout = layout_text(&atlas, "AA", &LayoutOptions::default());
//! assert_eq!(layout.glyphs.len(), 2);
//! ```

pub mod atlas;
pub mod layout;
pub mod sdf;

pub use atlas::{AtlasBuilder, FontAtlas, Glyph};
pub use layout::{layout_text, LayoutOptions, PositionedGlyph, TextLayout};
pub use sdf::coverage_to_sdf;
