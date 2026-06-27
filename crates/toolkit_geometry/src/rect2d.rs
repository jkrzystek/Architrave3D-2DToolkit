use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A simple axis-aligned 2D bounding rectangle.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rect2D {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect2D {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }
    
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }
    
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }
    
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
    
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x &&
        p.y >= self.min.y && p.y <= self.max.y
    }
    
    pub fn expand(&mut self, amount: f32) {
        self.min -= Vec2::splat(amount);
        self.max += Vec2::splat(amount);
    }
}

impl Default for Rect2D {
    fn default() -> Self {
        Self {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        }
    }
}
