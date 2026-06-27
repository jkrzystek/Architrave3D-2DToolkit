use glam::Vec2;
use serde::{Deserialize, Serialize};
use toolkit_geometry::Rect2D;

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
    /// Subdivides an outer rectangle into a grid of inner rectangles based on the config.
    /// Returns the centers of each cell. The caller can then spawn logic (merging, probabilities, etc.).
    pub fn pack(outer: &Rect2D, config: &GridPackConfig) -> Vec<Rect2D> {
        let usable_width = outer.width() - 2.0 * config.margin;
        let usable_height = outer.height() - 2.0 * config.margin;
        
        if usable_width <= 0.0 || usable_height <= 0.0 {
            return Vec::new();
        }
        
        let cols = (usable_width / (config.item_width + config.spacing_x)).floor() as usize;
        let rows = (usable_height / (config.item_height + config.spacing_y)).floor() as usize;
        
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
