//! [`Selection`]: a weighted set of element indices.
//!
//! Each selected element carries a weight in `[0, 1]`. A weight of `1` is a hard
//! selection; intermediate weights are a *soft selection* (proportional-edit
//! falloff), which tools multiply into their effect so a move/scale/paint tapers
//! off toward the edge of the selection. Indices are element-kind agnostic — the
//! same type selects points, edges, or faces.

use std::collections::BTreeMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use toolkit_brush::Falloff;

use crate::adjacency::Adjacency;

/// A set of element indices, each with a weight in `[0, 1]`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Selection {
    weights: BTreeMap<usize, f32>,
}

impl Selection {
    /// An empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a hard selection (weight `1`) from indices.
    pub fn from_indices(indices: impl IntoIterator<Item = usize>) -> Self {
        let mut s = Self::new();
        for i in indices {
            s.set(i, 1.0);
        }
        s
    }

    /// Number of selected elements (weight `> 0`).
    pub fn len(&self) -> usize {
        self.weights.len()
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.weights.is_empty()
    }

    /// Weight of element `i` (`0` if unselected).
    pub fn weight(&self, i: usize) -> f32 {
        self.weights.get(&i).copied().unwrap_or(0.0)
    }

    /// Whether `i` has any weight.
    pub fn contains(&self, i: usize) -> bool {
        self.weights.contains_key(&i)
    }

    /// Set the weight of `i`. A weight `<= 0` removes it; weights clamp to `1`.
    pub fn set(&mut self, i: usize, weight: f32) {
        let w = weight.min(1.0);
        if w <= 0.0 {
            self.weights.remove(&i);
        } else {
            self.weights.insert(i, w);
        }
    }

    /// Hard-select `i` (weight `1`).
    pub fn add(&mut self, i: usize) {
        self.set(i, 1.0);
    }

    /// Remove `i` from the selection.
    pub fn remove(&mut self, i: usize) {
        self.weights.remove(&i);
    }

    /// Clear the selection.
    pub fn clear(&mut self) {
        self.weights.clear();
    }

    /// Iterate `(index, weight)` pairs in ascending index order.
    pub fn iter(&self) -> impl Iterator<Item = (usize, f32)> + '_ {
        self.weights.iter().map(|(&i, &w)| (i, w))
    }

    /// Selected indices in ascending order.
    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.weights.keys().copied()
    }

    // -- Boolean ops ---------------------------------------------------------

    /// Union with another selection, keeping the larger weight per element.
    pub fn union(&self, other: &Selection) -> Selection {
        let mut out = self.clone();
        for (i, w) in other.iter() {
            if w > out.weight(i) {
                out.set(i, w);
            }
        }
        out
    }

    /// Intersection, keeping the smaller weight where both contain an element.
    pub fn intersect(&self, other: &Selection) -> Selection {
        let mut out = Selection::new();
        for (i, w) in self.iter() {
            let ow = other.weight(i);
            if ow > 0.0 {
                out.set(i, w.min(ow));
            }
        }
        out
    }

    /// Difference: this selection minus everything in `other`.
    pub fn difference(&self, other: &Selection) -> Selection {
        let mut out = self.clone();
        for i in other.indices() {
            out.remove(i);
        }
        out
    }

    /// Invert over a domain of `count` elements: every index gets `1 - weight`.
    pub fn invert(&self, count: usize) -> Selection {
        let mut out = Selection::new();
        for i in 0..count {
            out.set(i, 1.0 - self.weight(i));
        }
        out
    }

    // -- Topology growth -----------------------------------------------------

    /// Grow by one ring: every neighbour of a selected element joins, inheriting
    /// the strongest neighbouring weight. Existing weights are preserved.
    pub fn grow(&self, adjacency: &Adjacency) -> Selection {
        let mut out = self.clone();
        for (i, w) in self.iter() {
            for &n in adjacency.neighbors(i) {
                if w > out.weight(n) {
                    out.set(n, w);
                }
            }
        }
        out
    }

    /// Shrink by one ring: drop any selected element that has an unselected
    /// neighbour (i.e. keep only the interior).
    pub fn shrink(&self, adjacency: &Adjacency) -> Selection {
        let mut out = Selection::new();
        for (i, w) in self.iter() {
            let interior = adjacency.neighbors(i).iter().all(|&n| self.contains(n));
            if interior {
                out.set(i, w);
            }
        }
        out
    }

    // -- Construction from data ----------------------------------------------

    /// Select indices whose attribute value falls in `[min, max]` (weight `1`).
    pub fn from_threshold(values: &[f32], min: f32, max: f32) -> Selection {
        let mut out = Selection::new();
        for (i, &v) in values.iter().enumerate() {
            if v >= min && v <= max {
                out.add(i);
            }
        }
        out
    }

    /// Build a soft selection by distance: each element within `radius` of any
    /// `seed` element gets a weight from `falloff` based on its nearest-seed
    /// distance. `positions` indexes the whole domain; `seeds` are the hard core.
    pub fn from_falloff(
        seeds: &[usize],
        positions: &[Vec3],
        radius: f32,
        falloff: Falloff,
    ) -> Selection {
        let mut out = Selection::new();
        if radius <= 0.0 {
            return Selection::from_indices(seeds.iter().copied());
        }
        let seed_pts: Vec<Vec3> = seeds.iter().filter_map(|&s| positions.get(s).copied()).collect();
        for (i, &p) in positions.iter().enumerate() {
            let nearest = seed_pts
                .iter()
                .map(|&s| s.distance(p))
                .fold(f32::INFINITY, f32::min);
            if nearest <= radius {
                out.set(i, falloff.weight(nearest / radius));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_weight_clamp() {
        let mut s = Selection::new();
        s.set(0, 2.0); // clamps to 1
        s.set(1, 0.5);
        s.set(2, -1.0); // removes / ignored
        assert_eq!(s.weight(0), 1.0);
        assert_eq!(s.weight(1), 0.5);
        assert!(!s.contains(2));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn boolean_ops() {
        let a = Selection::from_indices([0, 1, 2]);
        let mut b = Selection::new();
        b.set(1, 0.5);
        b.set(3, 1.0);
        assert_eq!(a.union(&b).len(), 4);
        let inter = a.intersect(&b);
        assert_eq!(inter.len(), 1);
        assert_eq!(inter.weight(1), 0.5); // min of 1.0 and 0.5
        let diff = a.difference(&b);
        assert!(!diff.contains(1));
        assert_eq!(diff.len(), 2);
    }

    #[test]
    fn invert_over_domain() {
        let mut s = Selection::new();
        s.set(0, 1.0);
        s.set(1, 0.25);
        let inv = s.invert(3);
        assert_eq!(inv.weight(0), 0.0); // was fully selected
        assert!((inv.weight(1) - 0.75).abs() < 1e-6);
        assert_eq!(inv.weight(2), 1.0); // was unselected
    }

    #[test]
    fn grow_and_shrink() {
        // Path graph 0-1-2-3-4; select the middle vertex.
        let adj = Adjacency::from_pairs(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]);
        let s = Selection::from_indices([2]);
        let grown = s.grow(&adj);
        assert!(grown.contains(1) && grown.contains(3));
        assert_eq!(grown.len(), 3);
        // Shrinking the grown set removes the boundary (1 and 3 touch unselected).
        let shrunk = grown.shrink(&adj);
        assert!(shrunk.contains(2));
        assert_eq!(shrunk.len(), 1);
    }

    #[test]
    fn threshold_selects_in_range() {
        let s = Selection::from_threshold(&[0.0, 0.5, 1.0, 1.5], 0.4, 1.1);
        assert!(s.contains(1) && s.contains(2));
        assert!(!s.contains(0) && !s.contains(3));
    }

    #[test]
    fn soft_falloff_tapers_with_distance() {
        let positions = vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let s = Selection::from_falloff(&[0], &positions, 2.0, Falloff::Linear);
        assert_eq!(s.weight(0), 1.0); // at the seed
        assert!((s.weight(1) - 0.5).abs() < 1e-6); // halfway
        assert_eq!(s.weight(2), 0.0); // at the radius edge
    }

    #[test]
    fn serde_roundtrip() {
        let mut s = Selection::new();
        s.set(3, 0.7);
        let json = serde_json::to_string(&s).unwrap();
        let back: Selection = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
