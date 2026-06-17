//! A point octree over an axis-aligned region. Good for range/box queries and
//! culling when points cluster unevenly in space.

use glam::Vec3;
use toolkit_geometry::Aabb;

/// An octree of points within a fixed bounding box. Nodes subdivide once they
/// exceed `capacity`, down to `max_depth`.
pub struct Octree {
    bounds: Aabb,
    capacity: usize,
    max_depth: u8,
    depth: u8,
    points: Vec<(u32, Vec3)>,
    children: Option<Box<[Octree; 8]>>,
}

impl Octree {
    pub fn new(bounds: Aabb, capacity: usize, max_depth: u8) -> Self {
        Self {
            bounds,
            capacity: capacity.max(1),
            max_depth,
            depth: 0,
            points: Vec::new(),
            children: None,
        }
    }

    /// Build directly from a point slice, assigning ids `0..points.len()`.
    pub fn from_points(points: &[Vec3], capacity: usize, max_depth: u8) -> Self {
        let mut bounds = Aabb::from_points(points.iter().copied());
        // Pad slightly so boundary points are strictly contained.
        let pad = (bounds.max - bounds.min).max_element().max(1.0) * 1e-3;
        bounds.min -= Vec3::splat(pad);
        bounds.max += Vec3::splat(pad);
        let mut tree = Octree::new(bounds, capacity, max_depth);
        for (i, &p) in points.iter().enumerate() {
            tree.insert(i as u32, p);
        }
        tree
    }

    fn with_depth(bounds: Aabb, capacity: usize, max_depth: u8, depth: u8) -> Self {
        Self {
            bounds,
            capacity,
            max_depth,
            depth,
            points: Vec::new(),
            children: None,
        }
    }

    /// Insert a point with the given id. Returns `false` if it's outside bounds.
    pub fn insert(&mut self, id: u32, pos: Vec3) -> bool {
        if !self.bounds.contains_point(pos) {
            return false;
        }
        if let Some(children) = &mut self.children {
            let octant = Self::octant_index(&self.bounds, pos);
            return children[octant].insert(id, pos);
        }
        self.points.push((id, pos));
        if self.points.len() > self.capacity && self.depth < self.max_depth {
            self.subdivide();
        }
        true
    }

    fn subdivide(&mut self) {
        let c = self.bounds.center();
        let min = self.bounds.min;
        let max = self.bounds.max;
        let make = |lo: Vec3, hi: Vec3| {
            Octree::with_depth(Aabb::new(lo, hi), self.capacity, self.max_depth, self.depth + 1)
        };
        // 8 octants ordered by (x,y,z) low/high bits — matches octant_index.
        let children = Box::new([
            make(Vec3::new(min.x, min.y, min.z), Vec3::new(c.x, c.y, c.z)),
            make(Vec3::new(c.x, min.y, min.z), Vec3::new(max.x, c.y, c.z)),
            make(Vec3::new(min.x, c.y, min.z), Vec3::new(c.x, max.y, c.z)),
            make(Vec3::new(c.x, c.y, min.z), Vec3::new(max.x, max.y, c.z)),
            make(Vec3::new(min.x, min.y, c.z), Vec3::new(c.x, c.y, max.z)),
            make(Vec3::new(c.x, min.y, c.z), Vec3::new(max.x, c.y, max.z)),
            make(Vec3::new(min.x, c.y, c.z), Vec3::new(c.x, max.y, max.z)),
            make(Vec3::new(c.x, c.y, c.z), Vec3::new(max.x, max.y, max.z)),
        ]);
        self.children = Some(children);
        let pts = std::mem::take(&mut self.points);
        for (id, pos) in pts {
            let octant = Self::octant_index(&self.bounds, pos);
            self.children.as_mut().unwrap()[octant].insert(id, pos);
        }
    }

    fn octant_index(bounds: &Aabb, pos: Vec3) -> usize {
        let c = bounds.center();
        (pos.x >= c.x) as usize | ((pos.y >= c.y) as usize) << 1 | ((pos.z >= c.z) as usize) << 2
    }

    /// Ids of all points inside the query box.
    pub fn query_aabb(&self, query: &Aabb) -> Vec<u32> {
        let mut out = Vec::new();
        self.query_aabb_into(query, &mut out);
        out
    }

    fn query_aabb_into(&self, query: &Aabb, out: &mut Vec<u32>) {
        if !self.bounds.intersects(query) {
            return;
        }
        for &(id, pos) in &self.points {
            if query.contains_point(pos) {
                out.push(id);
            }
        }
        if let Some(children) = &self.children {
            for child in children.iter() {
                child.query_aabb_into(query, out);
            }
        }
    }

    /// Ids of all points within `radius` of `center`.
    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<u32> {
        let box_query = Aabb::new(center - Vec3::splat(radius), center + Vec3::splat(radius));
        let r2 = radius * radius;
        self.query_aabb(&box_query)
            .into_iter()
            .filter(|&id| self.find(id).map(|p| p.distance_squared(center) <= r2).unwrap_or(false))
            .collect()
    }

    fn find(&self, id: u32) -> Option<Vec3> {
        for &(pid, pos) in &self.points {
            if pid == id {
                return Some(pos);
            }
        }
        if let Some(children) = &self.children {
            for c in children.iter() {
                if let Some(p) = c.find(id) {
                    return Some(p);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_points() -> Vec<Vec3> {
        let mut pts = Vec::new();
        for x in 0..10 {
            for y in 0..10 {
                pts.push(Vec3::new(x as f32, y as f32, 0.0));
            }
        }
        pts
    }

    #[test]
    fn query_box_returns_contained_points() {
        let pts = grid_points();
        let tree = Octree::from_points(&pts, 4, 8);
        let query = Aabb::new(Vec3::new(2.0, 2.0, -0.5), Vec3::new(4.0, 4.0, 0.5));
        let ids = tree.query_aabb(&query);
        // x,y in {2,3,4} -> 9 points.
        assert_eq!(ids.len(), 9);
        for id in ids {
            let p = pts[id as usize];
            assert!(p.x >= 2.0 && p.x <= 4.0 && p.y >= 2.0 && p.y <= 4.0);
        }
    }

    #[test]
    fn radius_query_matches_brute_force() {
        let pts = grid_points();
        let tree = Octree::from_points(&pts, 4, 8);
        let center = Vec3::new(5.0, 5.0, 0.0);
        let radius = 2.0;
        let mut got = tree.query_radius(center, radius);
        let mut expected: Vec<u32> = (0..pts.len() as u32)
            .filter(|&i| pts[i as usize].distance(center) <= radius)
            .collect();
        got.sort();
        expected.sort();
        assert_eq!(got, expected);
    }

    #[test]
    fn point_outside_bounds_rejected() {
        let mut tree = Octree::new(Aabb::new(Vec3::ZERO, Vec3::ONE), 4, 4);
        assert!(tree.insert(0, Vec3::new(0.5, 0.5, 0.5)));
        assert!(!tree.insert(1, Vec3::new(5.0, 5.0, 5.0)));
    }
}
