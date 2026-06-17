//! A 3D spatial hash grid for fast radius / neighbourhood queries over a
//! dynamic point set (particles, collision broad-phase, point clouds).

use std::collections::HashMap;

use glam::Vec3;

/// A uniform spatial hash grid. Points are bucketed by cell; radius queries only
/// scan the cells overlapping the query sphere.
#[derive(Clone, Debug)]
pub struct SpatialHashGrid {
    cell_size: f32,
    inv_cell: f32,
    cells: HashMap<(i32, i32, i32), Vec<u32>>,
    points: Vec<Vec3>,
}

impl SpatialHashGrid {
    /// Create a grid with the given cell size (≈ your typical query radius).
    pub fn new(cell_size: f32) -> Self {
        let cell_size = cell_size.max(1e-6);
        Self {
            cell_size,
            inv_cell: 1.0 / cell_size,
            cells: HashMap::new(),
            points: Vec::new(),
        }
    }

    fn key(&self, p: Vec3) -> (i32, i32, i32) {
        (
            (p.x * self.inv_cell).floor() as i32,
            (p.y * self.inv_cell).floor() as i32,
            (p.z * self.inv_cell).floor() as i32,
        )
    }

    /// Insert a point, returning its assigned id.
    pub fn insert(&mut self, p: Vec3) -> u32 {
        let id = self.points.len() as u32;
        self.points.push(p);
        self.cells.entry(self.key(p)).or_default().push(id);
        id
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.points.clear();
    }

    pub fn point(&self, id: u32) -> Vec3 {
        self.points[id as usize]
    }

    /// Ids of all points within `radius` of `center`.
    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<u32> {
        let r2 = radius * radius;
        let min = self.key(center - Vec3::splat(radius));
        let max = self.key(center + Vec3::splat(radius));
        let mut out = Vec::new();
        for cx in min.0..=max.0 {
            for cy in min.1..=max.1 {
                for cz in min.2..=max.2 {
                    if let Some(ids) = self.cells.get(&(cx, cy, cz)) {
                        for &id in ids {
                            if self.points[id as usize].distance_squared(center) <= r2 {
                                out.push(id);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// The nearest point to `query` found by expanding ring-by-ring, or `None`
    /// if the grid is empty.
    pub fn nearest(&self, query: Vec3) -> Option<u32> {
        if self.is_empty() {
            return None;
        }
        let mut radius = self.cell_size;
        for _ in 0..64 {
            let candidates = self.query_radius(query, radius);
            if let Some(best) = candidates
                .into_iter()
                .min_by(|&a, &b| {
                    self.points[a as usize]
                        .distance_squared(query)
                        .total_cmp(&self.points[b as usize].distance_squared(query))
                })
            {
                return Some(best);
            }
            radius *= 2.0;
        }
        // Fallback: linear scan (degenerate distributions).
        (0..self.points.len() as u32).min_by(|&a, &b| {
            self.points[a as usize]
                .distance_squared(query)
                .total_cmp(&self.points[b as usize].distance_squared(query))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_query_finds_neighbours() {
        let mut grid = SpatialHashGrid::new(1.0);
        let a = grid.insert(Vec3::new(0.0, 0.0, 0.0));
        let _b = grid.insert(Vec3::new(0.5, 0.0, 0.0));
        let _c = grid.insert(Vec3::new(10.0, 0.0, 0.0));
        let near = grid.query_radius(Vec3::ZERO, 1.0);
        assert!(near.contains(&a));
        assert_eq!(near.len(), 2); // a and b, not c
    }

    #[test]
    fn radius_query_excludes_far_points() {
        let mut grid = SpatialHashGrid::new(2.0);
        grid.insert(Vec3::new(5.0, 5.0, 5.0));
        assert!(grid.query_radius(Vec3::ZERO, 1.0).is_empty());
    }

    #[test]
    fn nearest_matches_brute_force() {
        let mut grid = SpatialHashGrid::new(1.0);
        let pts = [
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(0.2, 0.1, 0.0),
            Vec3::new(-4.0, 2.0, 1.0),
        ];
        for p in pts {
            grid.insert(p);
        }
        let n = grid.nearest(Vec3::ZERO).unwrap();
        assert_eq!(grid.point(n), pts[1]);
    }

    #[test]
    fn clear_empties() {
        let mut grid = SpatialHashGrid::new(1.0);
        grid.insert(Vec3::ZERO);
        grid.clear();
        assert!(grid.is_empty());
        assert!(grid.nearest(Vec3::ZERO).is_none());
    }
}
