//! The wave-function-collapse solve: observe the lowest-entropy cell, collapse
//! it to a single weighted tile, and propagate the constraints, repeating until
//! every cell is decided or a contradiction is hit.

use toolkit_rng::Rng;

use crate::model::{Dir, WfcModel};

/// A solved grid of tile ids, row-major (`index = y * width + x`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WfcGrid {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<usize>,
}

impl WfcGrid {
    pub fn get(&self, x: usize, y: usize) -> usize {
        self.tiles[y * self.width + x]
    }
}

/// Run WFC over a `width`×`height` grid. Returns `None` on a contradiction
/// (no retry). Deterministic for a given `rng` seed.
pub fn solve(model: &WfcModel, width: usize, height: usize, rng: &mut Rng) -> Option<WfcGrid> {
    let n = model.tile_count();
    if n == 0 {
        return None;
    }
    if width == 0 || height == 0 {
        return Some(WfcGrid {
            width,
            height,
            tiles: Vec::new(),
        });
    }

    let adjacency = model.adjacency();
    let cells_len = width * height;
    // possibilities[cell] = bitmask over tiles (true = still possible).
    let mut poss = vec![vec![true; n]; cells_len];

    loop {
        // Pick the undecided cell with the fewest remaining possibilities.
        let mut target: Option<usize> = None;
        let mut best = usize::MAX;
        for (i, cell) in poss.iter().enumerate() {
            let count = cell.iter().filter(|&&b| b).count();
            if count == 0 {
                return None; // contradiction
            }
            if count > 1 && count < best {
                best = count;
                target = Some(i);
            }
        }

        let Some(cell) = target else {
            // Everything is collapsed; read off the single possibility per cell.
            let tiles = poss
                .iter()
                .map(|c| c.iter().position(|&b| b).unwrap())
                .collect();
            return Some(WfcGrid { width, height, tiles });
        };

        collapse(&mut poss[cell], model, rng);
        if !propagate(&mut poss, &adjacency, width, height, cell, n) {
            return None; // contradiction during propagation
        }
    }
}

/// Collapse a cell to a single tile chosen by weight among its possibilities.
fn collapse(cell: &mut [bool], model: &WfcModel, rng: &mut Rng) {
    let total: f32 = (0..cell.len())
        .filter(|&t| cell[t])
        .map(|t| model.weight(t))
        .sum();

    let mut chosen = (0..cell.len()).find(|&t| cell[t]).unwrap();
    if total > 0.0 {
        let mut pick = rng.range_f32(0.0, total);
        for t in 0..cell.len() {
            if cell[t] {
                pick -= model.weight(t);
                if pick <= 0.0 {
                    chosen = t;
                    break;
                }
            }
        }
    }
    for t in 0..cell.len() {
        cell[t] = t == chosen;
    }
}

/// Propagate constraints outward from `start`. Returns `false` on contradiction.
fn propagate(
    poss: &mut [Vec<bool>],
    adjacency: &[Vec<Vec<bool>>],
    width: usize,
    height: usize,
    start: usize,
    n: usize,
) -> bool {
    let mut stack = vec![start];
    while let Some(c) = stack.pop() {
        let (cx, cy) = ((c % width) as i32, (c / width) as i32);
        for dir in Dir::ALL {
            let (dx, dy) = dir.delta();
            let (nx, ny) = (cx + dx, cy + dy);
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let neighbor = (ny as usize) * width + nx as usize;

            // Tiles the neighbour may still be: those compatible with at least
            // one tile still possible in `c` (looking in `dir`).
            let mut changed = false;
            for nt in 0..n {
                if !poss[neighbor][nt] {
                    continue;
                }
                let supported = (0..n).any(|t| poss[c][t] && adjacency[dir as usize][t][nt]);
                if !supported {
                    poss[neighbor][nt] = false;
                    changed = true;
                }
            }
            if changed {
                if poss[neighbor].iter().all(|&b| !b) {
                    return false; // neighbour has no options left
                }
                stack.push(neighbor);
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two tiles that may sit next to each other in any arrangement.
    fn free_model() -> WfcModel {
        let mut m = WfcModel::new();
        let a = m.add_tile(1.0);
        let b = m.add_tile(1.0);
        for &x in &[a, b] {
            for &y in &[a, b] {
                m.allow(x, Dir::Right, y);
                m.allow(x, Dir::Up, y);
            }
        }
        m
    }

    #[test]
    fn fills_grid_completely() {
        let m = free_model();
        let mut rng = Rng::seed_from_u64(1);
        let grid = solve(&m, 5, 4, &mut rng).unwrap();
        assert_eq!(grid.tiles.len(), 20);
        assert!(grid.tiles.iter().all(|&t| t < 2));
    }

    #[test]
    fn deterministic_for_seed() {
        let m = free_model();
        let a = solve(&m, 6, 6, &mut Rng::seed_from_u64(7)).unwrap();
        let b = solve(&m, 6, 6, &mut Rng::seed_from_u64(7)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn respects_adjacency_constraints() {
        // Checkerboard: tile A only allows B beside it and vice versa.
        let mut m = WfcModel::new();
        let a = m.add_tile(1.0);
        let b = m.add_tile(1.0);
        m.allow(a, Dir::Right, b);
        m.allow(a, Dir::Up, b);
        // (symmetry gives B->A on the opposite sides; also need B beside A the
        // other way for a consistent checkerboard)
        m.allow(b, Dir::Right, a);
        m.allow(b, Dir::Up, a);

        let mut rng = Rng::seed_from_u64(3);
        let grid = solve(&m, 4, 4, &mut rng).unwrap();
        // No two horizontally/vertically adjacent cells share a tile.
        for y in 0..4 {
            for x in 0..4 {
                let t = grid.get(x, y);
                if x + 1 < 4 {
                    assert_ne!(t, grid.get(x + 1, y));
                }
                if y + 1 < 4 {
                    assert_ne!(t, grid.get(x, y + 1));
                }
            }
        }
    }

    #[test]
    fn single_tile_fills_uniformly() {
        let mut m = WfcModel::new();
        let a = m.add_tile(1.0);
        m.allow(a, Dir::Right, a);
        m.allow(a, Dir::Up, a);
        let grid = solve(&m, 3, 3, &mut Rng::seed_from_u64(0)).unwrap();
        assert!(grid.tiles.iter().all(|&t| t == a));
    }

    #[test]
    fn empty_grid_is_ok() {
        let m = free_model();
        let grid = solve(&m, 0, 0, &mut Rng::seed_from_u64(0)).unwrap();
        assert!(grid.tiles.is_empty());
    }
}
