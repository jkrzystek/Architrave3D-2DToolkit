//! A static 3D k-d tree for nearest-neighbour and radius queries over a fixed
//! point set. Build once, query many times.

use glam::Vec3;

#[derive(Clone, Copy)]
struct Node {
    point: usize,
    axis: u8,
    left: i32,
    right: i32,
}

/// An immutable k-d tree built from a slice of points. Queries return indices
/// into the original point array.
pub struct KdTree {
    points: Vec<Vec3>,
    nodes: Vec<Node>,
    root: i32,
}

impl KdTree {
    /// Build a balanced tree by recursively splitting on the median along
    /// cycling x/y/z axes.
    pub fn build(points: &[Vec3]) -> Self {
        let mut indices: Vec<usize> = (0..points.len()).collect();
        let mut nodes = Vec::with_capacity(points.len());
        let root = build_recursive(points, &mut indices, 0, &mut nodes);
        Self {
            points: points.to_vec(),
            nodes,
            root,
        }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Index of the point nearest to `query`, or `None` if the tree is empty.
    pub fn nearest(&self, query: Vec3) -> Option<usize> {
        if self.root < 0 {
            return None;
        }
        let mut best = usize::MAX;
        let mut best_d2 = f32::INFINITY;
        self.nearest_recursive(self.root, query, &mut best, &mut best_d2);
        Some(best)
    }

    fn nearest_recursive(&self, node: i32, query: Vec3, best: &mut usize, best_d2: &mut f32) {
        if node < 0 {
            return;
        }
        let n = self.nodes[node as usize];
        let p = self.points[n.point];
        let d2 = p.distance_squared(query);
        if d2 < *best_d2 {
            *best_d2 = d2;
            *best = n.point;
        }
        let axis = n.axis as usize;
        let diff = query[axis] - p[axis];
        let (near, far) = if diff < 0.0 {
            (n.left, n.right)
        } else {
            (n.right, n.left)
        };
        self.nearest_recursive(near, query, best, best_d2);
        // Only descend the far side if the splitting plane is within best radius.
        if diff * diff < *best_d2 {
            self.nearest_recursive(far, query, best, best_d2);
        }
    }

    /// Indices of all points within `radius` of `query`.
    pub fn within_radius(&self, query: Vec3, radius: f32) -> Vec<usize> {
        let mut out = Vec::new();
        self.radius_recursive(self.root, query, radius, radius * radius, &mut out);
        out
    }

    fn radius_recursive(&self, node: i32, query: Vec3, radius: f32, r2: f32, out: &mut Vec<usize>) {
        if node < 0 {
            return;
        }
        let n = self.nodes[node as usize];
        let p = self.points[n.point];
        if p.distance_squared(query) <= r2 {
            out.push(n.point);
        }
        let axis = n.axis as usize;
        let diff = query[axis] - p[axis];
        if diff <= 0.0 || diff.abs() <= radius {
            self.radius_recursive(n.left, query, radius, r2, out);
        }
        if diff >= 0.0 || diff.abs() <= radius {
            self.radius_recursive(n.right, query, radius, r2, out);
        }
    }
}

fn build_recursive(
    points: &[Vec3],
    indices: &mut [usize],
    depth: usize,
    nodes: &mut Vec<Node>,
) -> i32 {
    if indices.is_empty() {
        return -1;
    }
    let axis = (depth % 3) as u8;
    indices.sort_by(|&a, &b| points[a][axis as usize].total_cmp(&points[b][axis as usize]));
    let mid = indices.len() / 2;
    let point = indices[mid];

    // Reserve this node's slot before recursing so children get later indices.
    let node_index = nodes.len();
    nodes.push(Node {
        point,
        axis,
        left: -1,
        right: -1,
    });

    let (left_slice, right_slice) = indices.split_at_mut(mid);
    let left = build_recursive(points, left_slice, depth + 1, nodes);
    let right = build_recursive(points, &mut right_slice[1..], depth + 1, nodes);
    nodes[node_index].left = left;
    nodes[node_index].right = right;
    node_index as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tiny deterministic LCG so tests need no rng dependency.
    fn lcg(state: &mut u64) -> f32 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((*state >> 33) as f32 / (1u32 << 31) as f32) - 1.0
    }

    fn random_points(n: usize, seed: u64) -> Vec<Vec3> {
        let mut s = seed;
        (0..n)
            .map(|_| Vec3::new(lcg(&mut s) * 10.0, lcg(&mut s) * 10.0, lcg(&mut s) * 10.0))
            .collect()
    }

    fn brute_nearest(points: &[Vec3], q: Vec3) -> usize {
        (0..points.len())
            .min_by(|&a, &b| {
                points[a]
                    .distance_squared(q)
                    .total_cmp(&points[b].distance_squared(q))
            })
            .unwrap()
    }

    #[test]
    fn nearest_matches_brute_force() {
        let pts = random_points(200, 12345);
        let tree = KdTree::build(&pts);
        let mut s = 999;
        for _ in 0..50 {
            let q = Vec3::new(lcg(&mut s) * 12.0, lcg(&mut s) * 12.0, lcg(&mut s) * 12.0);
            let expected = pts[brute_nearest(&pts, q)];
            let got = pts[tree.nearest(q).unwrap()];
            assert!(
                (expected.distance(q) - got.distance(q)).abs() < 1e-4,
                "kd nearest mismatch"
            );
        }
    }

    #[test]
    fn within_radius_matches_brute_force() {
        let pts = random_points(300, 42);
        let tree = KdTree::build(&pts);
        let q = Vec3::new(1.0, -2.0, 3.0);
        let radius = 4.0;
        let mut expected: Vec<usize> = (0..pts.len())
            .filter(|&i| pts[i].distance(q) <= radius)
            .collect();
        let mut got = tree.within_radius(q, radius);
        expected.sort();
        got.sort();
        assert_eq!(expected, got);
    }

    #[test]
    fn empty_tree() {
        let tree = KdTree::build(&[]);
        assert!(tree.is_empty());
        assert!(tree.nearest(Vec3::ZERO).is_none());
    }

    #[test]
    fn single_point() {
        let tree = KdTree::build(&[Vec3::new(5.0, 5.0, 5.0)]);
        assert_eq!(tree.nearest(Vec3::ZERO), Some(0));
    }
}
