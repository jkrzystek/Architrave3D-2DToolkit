//! [`Adjacency`]: a generic neighbour graph used to grow and shrink selections.
//!
//! It is just "element `i` is adjacent to these elements", so it works for mesh
//! vertices (connected by edges), faces (sharing edges), or any other graph the
//! caller builds.

use serde::{Deserialize, Serialize};

/// Undirected adjacency between integer-indexed elements.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Adjacency {
    neighbors: Vec<Vec<usize>>,
}

impl Adjacency {
    /// An adjacency over `count` elements with no connections yet.
    pub fn new(count: usize) -> Self {
        Self {
            neighbors: vec![Vec::new(); count],
        }
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.neighbors.len()
    }

    /// Whether there are no elements.
    pub fn is_empty(&self) -> bool {
        self.neighbors.is_empty()
    }

    /// Connect `a` and `b` (symmetric, deduplicated). Out-of-range indices are
    /// ignored.
    pub fn connect(&mut self, a: usize, b: usize) {
        if a == b || a >= self.neighbors.len() || b >= self.neighbors.len() {
            return;
        }
        if !self.neighbors[a].contains(&b) {
            self.neighbors[a].push(b);
        }
        if !self.neighbors[b].contains(&a) {
            self.neighbors[b].push(a);
        }
    }

    /// Build from a list of undirected pairs.
    pub fn from_pairs(count: usize, pairs: &[(usize, usize)]) -> Self {
        let mut a = Self::new(count);
        for &(x, y) in pairs {
            a.connect(x, y);
        }
        a
    }

    /// Build vertex adjacency from polygon faces: consecutive vertices in each
    /// face loop are connected.
    pub fn vertex_from_faces(vertex_count: usize, faces: &[Vec<usize>]) -> Self {
        let mut a = Self::new(vertex_count);
        for f in faces {
            let k = f.len();
            for i in 0..k {
                a.connect(f[i], f[(i + 1) % k]);
            }
        }
        a
    }

    /// Neighbours of `i` (empty if out of range).
    pub fn neighbors(&self, i: usize) -> &[usize] {
        self.neighbors.get(i).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_is_symmetric_and_dedup() {
        let mut a = Adjacency::new(3);
        a.connect(0, 1);
        a.connect(0, 1); // duplicate ignored
        assert_eq!(a.neighbors(0), &[1]);
        assert_eq!(a.neighbors(1), &[0]);
        a.connect(0, 0); // self ignored
        assert_eq!(a.neighbors(0), &[1]);
    }

    #[test]
    fn from_faces_connects_ring() {
        // One quad 0-1-2-3.
        let a = Adjacency::vertex_from_faces(4, &[vec![0, 1, 2, 3]]);
        assert!(a.neighbors(0).contains(&1));
        assert!(a.neighbors(0).contains(&3));
        assert!(!a.neighbors(0).contains(&2)); // diagonal not adjacent
    }

    #[test]
    fn serde_roundtrip() {
        let a = Adjacency::from_pairs(3, &[(0, 1), (1, 2)]);
        let json = serde_json::to_string(&a).unwrap();
        let back: Adjacency = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }
}
