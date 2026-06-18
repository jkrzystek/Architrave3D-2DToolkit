//! Named colour collections plus helpers for generating evenly distributed
//! palettes (useful for category colours, charts, and editor swatches).

use serde::{Deserialize, Serialize};
use toolkit_core::LinearRgba;

use crate::hsv::Hsv;

/// An ordered set of named colours.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub colors: Vec<LinearRgba>,
}

impl Palette {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            colors: Vec::new(),
        }
    }

    pub fn from_colors(name: impl Into<String>, colors: Vec<LinearRgba>) -> Self {
        Self {
            name: name.into(),
            colors,
        }
    }

    pub fn push(&mut self, color: LinearRgba) -> &mut Self {
        self.colors.push(color);
        self
    }

    pub fn len(&self) -> usize {
        self.colors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// Fetch a colour, wrapping the index so any `i` is valid (handy for
    /// assigning stable colours to an unbounded number of categories).
    pub fn get_wrapping(&self, i: usize) -> Option<LinearRgba> {
        if self.colors.is_empty() {
            None
        } else {
            Some(self.colors[i % self.colors.len()])
        }
    }

    /// Generate `count` maximally distinct hues at a fixed saturation/value,
    /// spaced evenly around the colour wheel. Good default category palette.
    pub fn distinct_hues(count: usize, saturation: f32, value: f32) -> Self {
        let colors = (0..count)
            .map(|i| {
                let hue = 360.0 * i as f32 / count.max(1) as f32;
                Hsv::new(hue, saturation, value, 1.0).to_linear()
            })
            .collect();
        Self::from_colors("distinct", colors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_index() {
        let p = Palette::from_colors("p", vec![LinearRgba::BLACK, LinearRgba::WHITE]);
        assert_eq!(p.get_wrapping(0), Some(LinearRgba::BLACK));
        assert_eq!(p.get_wrapping(3), Some(LinearRgba::WHITE));
    }

    #[test]
    fn empty_palette_get_is_none() {
        let p = Palette::new("empty");
        assert!(p.is_empty());
        assert_eq!(p.get_wrapping(0), None);
    }

    #[test]
    fn distinct_hues_count() {
        let p = Palette::distinct_hues(6, 0.7, 0.9);
        assert_eq!(p.len(), 6);
    }

    #[test]
    fn distinct_hues_are_different() {
        let p = Palette::distinct_hues(3, 1.0, 1.0);
        // Pairwise distinct colours.
        assert_ne!(p.colors[0], p.colors[1]);
        assert_ne!(p.colors[1], p.colors[2]);
    }

    #[test]
    fn serde_roundtrip() {
        let p = Palette::distinct_hues(4, 0.5, 0.8);
        let json = serde_json::to_string(&p).unwrap();
        let back: Palette = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
