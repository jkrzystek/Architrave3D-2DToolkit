//! Axis-aligned 2D rectangle utilities.
//!
//! [`Rect2D`] stores a min/max pair and exposes geometric queries such as
//! containment, intersection, union, and area.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A simple axis-aligned 2D bounding rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect2D {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect2D {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Create a rectangle from a center point and full size (width, height).
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half = size * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Returns `(width, height)` as a [`Vec2`].
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Returns the area of the rectangle. Negative if the rect is inverted.
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Returns `true` when `min <= max` on both axes.
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x &&
        p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn expand(&mut self, amount: f32) {
        self.min -= Vec2::splat(amount);
        self.max += Vec2::splat(amount);
        debug_assert!(
            self.width() >= 0.0 && self.height() >= 0.0,
            "expand() inverted the rect"
        );
    }

    /// Returns the smallest rectangle that contains both `self` and `other`.
    pub fn union(&self, other: &Rect2D) -> Rect2D {
        Rect2D {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Returns the overlapping rectangle, or `None` if the two rects do not
    /// overlap.
    pub fn intersection(&self, other: &Rect2D) -> Option<Rect2D> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min.x <= max.x && min.y <= max.y {
            Some(Rect2D { min, max })
        } else {
            None
        }
    }

    /// Returns `true` if `self` and `other` share any area (edges touching
    /// counts as overlapping).
    pub fn overlaps(&self, other: &Rect2D) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ────────────────────────────────────────────────────

    #[test]
    fn new_stores_min_max() {
        let r = Rect2D::new(Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0));
        assert_eq!(r.min, Vec2::new(1.0, 2.0));
        assert_eq!(r.max, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn from_center_size_basic() {
        let r = Rect2D::from_center_size(Vec2::new(5.0, 5.0), Vec2::new(4.0, 6.0));
        assert_eq!(r.min, Vec2::new(3.0, 2.0));
        assert_eq!(r.max, Vec2::new(7.0, 8.0));
    }

    #[test]
    fn from_center_size_zero() {
        let r = Rect2D::from_center_size(Vec2::ZERO, Vec2::ZERO);
        assert_eq!(r.min, Vec2::ZERO);
        assert_eq!(r.max, Vec2::ZERO);
    }

    #[test]
    fn default_is_zero_rect() {
        let r = Rect2D::default();
        assert_eq!(r.min, Vec2::ZERO);
        assert_eq!(r.max, Vec2::ZERO);
    }

    // ── Dimensions ──────────────────────────────────────────────────────

    #[test]
    fn width_and_height() {
        let r = Rect2D::new(Vec2::new(1.0, 2.0), Vec2::new(4.0, 7.0));
        assert_eq!(r.width(), 3.0);
        assert_eq!(r.height(), 5.0);
    }

    #[test]
    fn size_returns_vec2() {
        let r = Rect2D::new(Vec2::new(1.0, 2.0), Vec2::new(4.0, 7.0));
        assert_eq!(r.size(), Vec2::new(3.0, 5.0));
    }

    #[test]
    fn area_positive() {
        let r = Rect2D::new(Vec2::ZERO, Vec2::new(3.0, 4.0));
        assert_eq!(r.area(), 12.0);
    }

    #[test]
    fn area_zero_for_degenerate_line() {
        let r = Rect2D::new(Vec2::ZERO, Vec2::new(5.0, 0.0));
        assert_eq!(r.area(), 0.0);
    }

    #[test]
    fn area_zero_for_point() {
        let r = Rect2D::default();
        assert_eq!(r.area(), 0.0);
    }

    // ── Center ──────────────────────────────────────────────────────────

    #[test]
    fn center_of_unit_rect() {
        let r = Rect2D::new(Vec2::ZERO, Vec2::new(2.0, 4.0));
        assert_eq!(r.center(), Vec2::new(1.0, 2.0));
    }

    // ── Validity ────────────────────────────────────────────────────────

    #[test]
    fn is_valid_normal() {
        assert!(Rect2D::new(Vec2::ZERO, Vec2::new(1.0, 1.0)).is_valid());
    }

    #[test]
    fn is_valid_degenerate_point() {
        assert!(Rect2D::default().is_valid());
    }

    #[test]
    fn is_valid_inverted() {
        let r = Rect2D::new(Vec2::new(5.0, 5.0), Vec2::ZERO);
        assert!(!r.is_valid());
    }

    #[test]
    fn is_valid_inverted_one_axis() {
        let r = Rect2D::new(Vec2::new(5.0, 0.0), Vec2::new(0.0, 5.0));
        assert!(!r.is_valid());
    }

    // ── Contains ────────────────────────────────────────────────────────

    #[test]
    fn contains_interior_point() {
        let r = Rect2D::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(r.contains(Vec2::new(5.0, 5.0)));
    }

    #[test]
    fn contains_edge_point() {
        let r = Rect2D::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(r.contains(Vec2::new(0.0, 5.0)));
        assert!(r.contains(Vec2::new(10.0, 5.0)));
        assert!(r.contains(Vec2::new(5.0, 0.0)));
        assert!(r.contains(Vec2::new(5.0, 10.0)));
    }

    #[test]
    fn contains_corner_points() {
        let r = Rect2D::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(r.contains(Vec2::ZERO));
        assert!(r.contains(Vec2::new(10.0, 10.0)));
    }

    #[test]
    fn does_not_contain_outside() {
        let r = Rect2D::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(!r.contains(Vec2::new(-1.0, 5.0)));
        assert!(!r.contains(Vec2::new(11.0, 5.0)));
        assert!(!r.contains(Vec2::new(5.0, -1.0)));
        assert!(!r.contains(Vec2::new(5.0, 11.0)));
    }

    // ── Expand ──────────────────────────────────────────────────────────

    #[test]
    fn expand_positive() {
        let mut r = Rect2D::new(Vec2::new(2.0, 2.0), Vec2::new(8.0, 8.0));
        r.expand(1.0);
        assert_eq!(r.min, Vec2::new(1.0, 1.0));
        assert_eq!(r.max, Vec2::new(9.0, 9.0));
    }

    #[test]
    fn expand_zero_is_noop() {
        let mut r = Rect2D::new(Vec2::ZERO, Vec2::new(4.0, 4.0));
        let before = r;
        r.expand(0.0);
        assert_eq!(r, before);
    }

    #[test]
    fn expand_negative_within_bounds() {
        let mut r = Rect2D::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        r.expand(-2.0);
        assert_eq!(r.min, Vec2::new(2.0, 2.0));
        assert_eq!(r.max, Vec2::new(8.0, 8.0));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "expand() inverted the rect")]
    fn expand_negative_too_large_panics_in_debug() {
        let mut r = Rect2D::new(Vec2::ZERO, Vec2::new(2.0, 2.0));
        r.expand(-5.0);
    }

    // ── Union ───────────────────────────────────────────────────────────

    #[test]
    fn union_non_overlapping() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let b = Rect2D::new(Vec2::new(5.0, 5.0), Vec2::new(6.0, 6.0));
        let u = a.union(&b);
        assert_eq!(u.min, Vec2::ZERO);
        assert_eq!(u.max, Vec2::new(6.0, 6.0));
    }

    #[test]
    fn union_overlapping() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(3.0, 3.0));
        let b = Rect2D::new(Vec2::new(2.0, 2.0), Vec2::new(5.0, 5.0));
        let u = a.union(&b);
        assert_eq!(u.min, Vec2::ZERO);
        assert_eq!(u.max, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn union_contained() {
        let outer = Rect2D::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let inner = Rect2D::new(Vec2::new(2.0, 2.0), Vec2::new(4.0, 4.0));
        assert_eq!(outer.union(&inner), outer);
    }

    #[test]
    fn union_is_commutative() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let b = Rect2D::new(Vec2::new(3.0, 3.0), Vec2::new(5.0, 5.0));
        assert_eq!(a.union(&b), b.union(&a));
    }

    // ── Intersection ────────────────────────────────────────────────────

    #[test]
    fn intersection_overlapping() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(4.0, 4.0));
        let b = Rect2D::new(Vec2::new(2.0, 2.0), Vec2::new(6.0, 6.0));
        let i = a.intersection(&b).unwrap();
        assert_eq!(i.min, Vec2::new(2.0, 2.0));
        assert_eq!(i.max, Vec2::new(4.0, 4.0));
    }

    #[test]
    fn intersection_touching_edge() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let b = Rect2D::new(Vec2::new(2.0, 0.0), Vec2::new(4.0, 2.0));
        let i = a.intersection(&b).unwrap();
        // Touching edge → degenerate line rect
        assert_eq!(i.min, Vec2::new(2.0, 0.0));
        assert_eq!(i.max, Vec2::new(2.0, 2.0));
        assert_eq!(i.area(), 0.0);
    }

    #[test]
    fn intersection_touching_corner() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let b = Rect2D::new(Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0));
        let i = a.intersection(&b).unwrap();
        assert_eq!(i.min, Vec2::new(1.0, 1.0));
        assert_eq!(i.max, Vec2::new(1.0, 1.0));
    }

    #[test]
    fn intersection_none() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let b = Rect2D::new(Vec2::new(5.0, 5.0), Vec2::new(6.0, 6.0));
        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn intersection_contained() {
        let outer = Rect2D::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let inner = Rect2D::new(Vec2::new(2.0, 2.0), Vec2::new(4.0, 4.0));
        assert_eq!(outer.intersection(&inner).unwrap(), inner);
    }

    #[test]
    fn intersection_is_commutative() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(3.0, 3.0));
        let b = Rect2D::new(Vec2::new(1.0, 1.0), Vec2::new(5.0, 5.0));
        assert_eq!(a.intersection(&b), b.intersection(&a));
    }

    // ── Overlaps ────────────────────────────────────────────────────────

    #[test]
    fn overlaps_true() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(3.0, 3.0));
        let b = Rect2D::new(Vec2::new(2.0, 2.0), Vec2::new(5.0, 5.0));
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn overlaps_false() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let b = Rect2D::new(Vec2::new(5.0, 5.0), Vec2::new(6.0, 6.0));
        assert!(!a.overlaps(&b));
        assert!(!b.overlaps(&a));
    }

    #[test]
    fn overlaps_touching_edge() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let b = Rect2D::new(Vec2::new(2.0, 0.0), Vec2::new(4.0, 2.0));
        assert!(a.overlaps(&b));
    }

    #[test]
    fn overlaps_touching_corner() {
        let a = Rect2D::new(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let b = Rect2D::new(Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0));
        assert!(a.overlaps(&b));
    }

    // ── Round-trip: from_center_size ↔ center / size ────────────────────

    #[test]
    fn from_center_size_roundtrip() {
        let center = Vec2::new(7.0, -3.0);
        let size = Vec2::new(10.0, 6.0);
        let r = Rect2D::from_center_size(center, size);
        assert_eq!(r.center(), center);
        assert_eq!(r.size(), size);
    }
}
