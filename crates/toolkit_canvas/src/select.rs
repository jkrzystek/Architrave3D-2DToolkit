use glam::Vec2;
use serde::{Deserialize, Serialize};

/// An axis-aligned rectangle in canvas space. Stored as `min`/`max` with
/// `min <= max` on both axes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect2 {
    /// Build from any two opposite corners (order-independent).
    pub fn from_corners(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn area(&self) -> f32 {
        let s = self.size();
        (s.x * s.y).max(0.0)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn intersects(&self, other: &Rect2) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}

/// A rubber-band selection drag in canvas space. Tracks an anchor and the
/// current cursor; the resulting [`Rect2`] is what's used to select items.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SelectionDrag {
    anchor: Vec2,
    current: Vec2,
    active: bool,
}

impl SelectionDrag {
    pub fn begin(&mut self, canvas_point: Vec2) {
        self.anchor = canvas_point;
        self.current = canvas_point;
        self.active = true;
    }

    pub fn update(&mut self, canvas_point: Vec2) {
        if self.active {
            self.current = canvas_point;
        }
    }

    /// Finish the drag and return the selection rectangle (if it was active).
    pub fn finish(&mut self) -> Option<Rect2> {
        if !self.active {
            return None;
        }
        self.active = false;
        Some(Rect2::from_corners(self.anchor, self.current))
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The current rectangle while dragging (for live preview).
    pub fn rect(&self) -> Rect2 {
        Rect2::from_corners(self.anchor, self.current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_from_corners_normalizes() {
        let r = Rect2::from_corners(Vec2::new(5.0, 5.0), Vec2::new(1.0, 2.0));
        assert_eq!(r.min, Vec2::new(1.0, 2.0));
        assert_eq!(r.max, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn rect_contains_and_intersects() {
        let a = Rect2::from_corners(Vec2::ZERO, Vec2::splat(10.0));
        assert!(a.contains(Vec2::splat(5.0)));
        assert!(!a.contains(Vec2::splat(15.0)));
        let b = Rect2::from_corners(Vec2::splat(5.0), Vec2::splat(15.0));
        assert!(a.intersects(&b));
        let c = Rect2::from_corners(Vec2::splat(20.0), Vec2::splat(30.0));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn selection_drag_lifecycle() {
        let mut drag = SelectionDrag::default();
        assert!(!drag.is_active());
        drag.begin(Vec2::new(1.0, 1.0));
        drag.update(Vec2::new(4.0, 5.0));
        assert!(drag.is_active());
        let rect = drag.finish().unwrap();
        assert_eq!(rect.min, Vec2::new(1.0, 1.0));
        assert_eq!(rect.max, Vec2::new(4.0, 5.0));
        assert!(!drag.is_active());
        assert!(drag.finish().is_none());
    }
}
