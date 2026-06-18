//! The tile set and adjacency rules a solve runs against.

use serde::{Deserialize, Serialize};

/// Cardinal directions on the grid. `opposite()` gives the reciprocal needed to
/// keep adjacency symmetric.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dir {
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Right, Dir::Left, Dir::Up, Dir::Down];

    pub fn opposite(self) -> Dir {
        match self {
            Dir::Right => Dir::Left,
            Dir::Left => Dir::Right,
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
        }
    }

    /// Grid step `(dx, dy)` for this direction (y grows upward).
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Right => (1, 0),
            Dir::Left => (-1, 0),
            Dir::Up => (0, 1),
            Dir::Down => (0, -1),
        }
    }
}

/// A tile set plus the rules describing which tiles may sit next to which.
/// Rules are symmetric: [`WfcModel::allow`] records both directions.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WfcModel {
    weights: Vec<f32>,
    /// `(tile, dir, neighbor)`: `neighbor` may sit on the `dir` side of `tile`.
    rules: Vec<(usize, u8, usize)>,
}

impl WfcModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tile with a selection `weight` (higher = more common). Returns its id.
    pub fn add_tile(&mut self, weight: f32) -> usize {
        self.weights.push(weight.max(0.0));
        self.weights.len() - 1
    }

    pub fn tile_count(&self) -> usize {
        self.weights.len()
    }

    pub fn weight(&self, tile: usize) -> f32 {
        self.weights[tile]
    }

    /// Permit `neighbor` to sit on the `dir` side of `tile` (and, symmetrically,
    /// `tile` on the opposite side of `neighbor`).
    pub fn allow(&mut self, tile: usize, dir: Dir, neighbor: usize) -> &mut Self {
        self.rules.push((tile, dir as u8, neighbor));
        self.rules.push((neighbor, dir.opposite() as u8, tile));
        self
    }

    /// Build dense adjacency tables: `table[dir][tile][neighbor]`.
    pub(crate) fn adjacency(&self) -> Vec<Vec<Vec<bool>>> {
        let n = self.tile_count();
        let mut table = vec![vec![vec![false; n]; n]; 4];
        for &(tile, dir, neighbor) in &self.rules {
            table[dir as usize][tile][neighbor] = true;
        }
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opposite_directions() {
        assert_eq!(Dir::Right.opposite(), Dir::Left);
        assert_eq!(Dir::Up.opposite(), Dir::Down);
    }

    #[test]
    fn allow_is_symmetric() {
        let mut m = WfcModel::new();
        let a = m.add_tile(1.0);
        let b = m.add_tile(1.0);
        m.allow(a, Dir::Right, b);
        let adj = m.adjacency();
        assert!(adj[Dir::Right as usize][a][b]);
        assert!(adj[Dir::Left as usize][b][a]); // reciprocal recorded
    }

    #[test]
    fn serde_roundtrip() {
        let mut m = WfcModel::new();
        let a = m.add_tile(2.0);
        let b = m.add_tile(1.0);
        m.allow(a, Dir::Up, b);
        let json = serde_json::to_string(&m).unwrap();
        let back: WfcModel = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tile_count(), 2);
    }
}
