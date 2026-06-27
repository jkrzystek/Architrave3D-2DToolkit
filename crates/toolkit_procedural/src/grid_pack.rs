//! Grid packing utilities.
//!
//! Grid packing subdivides an axis-aligned outer rectangle into a regular grid
//! of equally-sized inner rectangles (cells). The layout respects configurable
//! item dimensions, inter-cell spacing, and an outer margin. This is useful for
//! procedurally placing windows, tiles, or other modular elements onto a façade
//! or surface.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use toolkit_geometry::Rect2D;

/// Configuration for a regular grid packing layout.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridPackConfig {
    pub item_width: f32,
    pub item_height: f32,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub margin: f32,
}

pub struct GridPacker;

impl GridPacker {
    /// Subdivides an outer rectangle into a grid of inner cell rectangles
    /// based on the config.
    ///
    /// Returns the [`Rect2D`] bounds of each cell. The caller can then apply
    /// additional logic (merging, probability filtering, etc.) on top.
    pub fn pack(outer: &Rect2D, config: &GridPackConfig) -> Vec<Rect2D> {
        let usable_width = outer.width() - 2.0 * config.margin;
        let usable_height = outer.height() - 2.0 * config.margin;
        
        if usable_width <= 0.0 || usable_height <= 0.0 {
            return Vec::new();
        }
        
        // `.max(0.0)` prevents a negative value from wrapping to a huge usize.
        let cols = (usable_width / (config.item_width + config.spacing_x)).floor().max(0.0) as usize;
        let rows = (usable_height / (config.item_height + config.spacing_y)).floor().max(0.0) as usize;
        
        let start_x = outer.min.x + config.margin;
        let start_y = outer.min.y + config.margin;
        
        let mut results = Vec::with_capacity(cols * rows);
        
        for r in 0..rows {
            for c in 0..cols {
                let x = start_x + c as f32 * (config.item_width + config.spacing_x);
                let y = start_y + r as f32 * (config.item_height + config.spacing_y);
                
                results.push(Rect2D::new(
                    Vec2::new(x, y),
                    Vec2::new(x + config.item_width, y + config.item_height)
                ));
            }
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect2D {
        Rect2D::new(Vec2::new(x0, y0), Vec2::new(x1, y1))
    }

    #[test]
    fn empty_when_margin_exceeds_bounds() {
        let outer = rect(0.0, 0.0, 10.0, 10.0);
        let config = GridPackConfig {
            item_width: 1.0,
            item_height: 1.0,
            spacing_x: 0.0,
            spacing_y: 0.0,
            margin: 6.0, // 2 * 6 = 12 > 10, usable area negative
        };
        let cells = GridPacker::pack(&outer, &config);
        assert!(cells.is_empty(), "Should produce no cells when margin exceeds bounds");
    }

    #[test]
    fn single_cell() {
        let outer = rect(0.0, 0.0, 10.0, 10.0);
        let config = GridPackConfig {
            item_width: 8.0,
            item_height: 8.0,
            spacing_x: 0.0,
            spacing_y: 0.0,
            margin: 1.0, // usable 8×8, fits exactly 1 cell
        };
        let cells = GridPacker::pack(&outer, &config);
        assert_eq!(cells.len(), 1);
        let c = &cells[0];
        assert!((c.min.x - 1.0).abs() < 1e-5);
        assert!((c.min.y - 1.0).abs() < 1e-5);
        assert!((c.max.x - 9.0).abs() < 1e-5);
        assert!((c.max.y - 9.0).abs() < 1e-5);
    }

    #[test]
    fn grid_layout_3x2() {
        let outer = rect(0.0, 0.0, 20.0, 14.0);
        let config = GridPackConfig {
            item_width: 4.0,
            item_height: 5.0,
            spacing_x: 1.0,
            spacing_y: 1.0,
            margin: 1.0,
            // usable: 18 x 12
            // cols = floor(18 / 5) = 3
            // rows = floor(12 / 6) = 2
        };
        let cells = GridPacker::pack(&outer, &config);
        assert_eq!(cells.len(), 6, "Expected 3 cols × 2 rows = 6 cells");

        // First cell starts at (margin, margin)
        let first = &cells[0];
        assert!((first.min.x - 1.0).abs() < 1e-5);
        assert!((first.min.y - 1.0).abs() < 1e-5);

        // Cell dimensions are correct
        assert!((first.width() - 4.0).abs() < 1e-5);
        assert!((first.height() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn no_negative_cols_on_tiny_items() {
        // Edge case: item + spacing is larger than usable area but usable area
        // itself is positive — should produce zero cells, not panic.
        let outer = rect(0.0, 0.0, 3.0, 3.0);
        let config = GridPackConfig {
            item_width: 10.0,
            item_height: 10.0,
            spacing_x: 0.0,
            spacing_y: 0.0,
            margin: 0.5, // usable 2×2, but items are 10×10
        };
        let cells = GridPacker::pack(&outer, &config);
        assert!(cells.is_empty(), "Items larger than usable area should yield no cells");
    }
}
