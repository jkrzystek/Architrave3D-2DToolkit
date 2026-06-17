use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::mesh::{Aabb, Mesh};

/// Maximum number of triangles in a BVH leaf node.
const LEAF_THRESHOLD: usize = 4;

/// Cost of traversing one internal node (SAH constant).
const TRAVERSAL_COST: f32 = 1.0;

// ---------------------------------------------------------------------------
// BVH tree types
// ---------------------------------------------------------------------------

/// A node in the bounding-volume hierarchy.
#[derive(Clone, Debug)]
pub enum BvhNode {
    Leaf {
        aabb: Aabb,
        triangle_indices: Vec<u32>,
    },
    Internal {
        aabb: Aabb,
        left: Box<BvhNode>,
        right: Box<BvhNode>,
    },
}

impl BvhNode {
    /// Return the AABB of this node regardless of variant.
    pub fn aabb(&self) -> &Aabb {
        match self {
            BvhNode::Leaf { aabb, .. } | BvhNode::Internal { aabb, .. } => aabb,
        }
    }
}

/// Bounding-volume hierarchy over a triangle mesh.
#[derive(Clone, Debug)]
pub struct Bvh {
    pub root: BvhNode,
}

impl Bvh {
    /// Build a BVH from a mesh using the Surface Area Heuristic.
    pub fn build(mesh: &Mesh) -> Self {
        let tri_count = mesh.triangle_count();
        if tri_count == 0 {
            return Self {
                root: BvhNode::Leaf {
                    aabb: Aabb::EMPTY,
                    triangle_indices: Vec::new(),
                },
            };
        }

        // Precompute per-triangle AABB and centroid.
        let mut tri_data: Vec<TriInfo> = Vec::with_capacity(tri_count);
        for tri_idx in 0..tri_count {
            let i0 = mesh.indices[tri_idx * 3] as usize;
            let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
            let i2 = mesh.indices[tri_idx * 3 + 2] as usize;

            let p0 = mesh.vertices[i0].position_vec3();
            let p1 = mesh.vertices[i1].position_vec3();
            let p2 = mesh.vertices[i2].position_vec3();

            let aabb = Aabb::from_points([p0, p1, p2]);
            let centroid = (p0 + p1 + p2) / 3.0;

            tri_data.push(TriInfo {
                index: tri_idx as u32,
                aabb,
                centroid,
            });
        }

        let root = build_recursive(&mut tri_data);
        Self { root }
    }

    /// Flatten the tree into a GPU-friendly linear array.
    pub fn flatten(&self) -> FlatBvh {
        let mut nodes = Vec::new();
        flatten_recursive(&self.root, &mut nodes);
        FlatBvh { nodes }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

struct TriInfo {
    index: u32,
    aabb: Aabb,
    centroid: Vec3,
}

fn build_recursive(tris: &mut [TriInfo]) -> BvhNode {
    // Compute overall AABB.
    let mut bounds = Aabb::EMPTY;
    for t in tris.iter() {
        bounds.expand_to_include_aabb(&t.aabb);
    }

    if tris.len() <= LEAF_THRESHOLD {
        return BvhNode::Leaf {
            aabb: bounds,
            triangle_indices: tris.iter().map(|t| t.index).collect(),
        };
    }

    // Compute centroid AABB for choosing a split plane.
    let centroid_bounds = Aabb::from_points(tris.iter().map(|t| t.centroid));
    let centroid_extents = centroid_bounds.extents();

    // Try splitting along each axis; pick the best SAH cost.
    let parent_area = bounds.surface_area().max(f32::EPSILON);
    let n = tris.len();

    let mut best_cost = f32::INFINITY;
    let mut best_axis = 0usize;
    let mut best_split = n / 2;

    for axis in 0..3 {
        let extent = match axis {
            0 => centroid_extents.x,
            1 => centroid_extents.y,
            _ => centroid_extents.z,
        };
        if extent < f32::EPSILON {
            continue; // All centroids coincide on this axis.
        }

        // Sort by centroid on the current axis.
        tris.sort_unstable_by(|a, b| {
            let ca = match axis {
                0 => a.centroid.x,
                1 => a.centroid.y,
                _ => a.centroid.z,
            };
            let cb = match axis {
                0 => b.centroid.x,
                1 => b.centroid.y,
                _ => b.centroid.z,
            };
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Sweep from left to build prefix AABBs.
        let mut left_areas = vec![0.0f32; n];
        {
            let mut left_aabb = Aabb::EMPTY;
            for i in 0..n {
                left_aabb.expand_to_include_aabb(&tris[i].aabb);
                left_areas[i] = left_aabb.surface_area();
            }
        }

        // Sweep from right.
        let mut right_areas = vec![0.0f32; n];
        {
            let mut right_aabb = Aabb::EMPTY;
            for i in (0..n).rev() {
                right_aabb.expand_to_include_aabb(&tris[i].aabb);
                right_areas[i] = right_aabb.surface_area();
            }
        }

        // Evaluate SAH at every possible split position.
        for split in 1..n {
            let left_count = split as f32;
            let right_count = (n - split) as f32;
            let cost = TRAVERSAL_COST
                + (left_areas[split - 1] / parent_area) * left_count
                + (right_areas[split] / parent_area) * right_count;

            if cost < best_cost {
                best_cost = cost;
                best_axis = axis;
                best_split = split;
            }
        }
    }

    // If no beneficial split was found, make a leaf.
    let leaf_cost = n as f32;
    if best_cost >= leaf_cost {
        return BvhNode::Leaf {
            aabb: bounds,
            triangle_indices: tris.iter().map(|t| t.index).collect(),
        };
    }

    // Sort along the best axis (may already be sorted if it was the last axis tried).
    tris.sort_unstable_by(|a, b| {
        let ca = match best_axis {
            0 => a.centroid.x,
            1 => a.centroid.y,
            _ => a.centroid.z,
        };
        let cb = match best_axis {
            0 => b.centroid.x,
            1 => b.centroid.y,
            _ => b.centroid.z,
        };
        ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let (left_tris, right_tris) = tris.split_at_mut(best_split);
    let left = build_recursive(left_tris);
    let right = build_recursive(right_tris);

    BvhNode::Internal {
        aabb: bounds,
        left: Box::new(left),
        right: Box::new(right),
    }
}

// ---------------------------------------------------------------------------
// Flat (GPU-friendly) BVH
// ---------------------------------------------------------------------------

/// A linearised BVH node suitable for GPU traversal.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FlatBvhNode {
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    /// For internal nodes: index of the left child.
    /// For leaf nodes: index of the first triangle in the triangle list.
    pub left_or_first: u32,
    /// For internal nodes: index of the right child.
    /// For leaf nodes: number of triangles.
    pub right_or_count: u32,
    /// 1 if this is a leaf node, 0 for internal.
    pub is_leaf: u32,
}

/// A flattened BVH stored as a contiguous array of [`FlatBvhNode`]s.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlatBvh {
    pub nodes: Vec<FlatBvhNode>,
}

fn flatten_recursive(node: &BvhNode, out: &mut Vec<FlatBvhNode>) {
    match node {
        BvhNode::Leaf {
            aabb,
            triangle_indices,
        } => {
            out.push(FlatBvhNode {
                aabb_min: aabb.min,
                aabb_max: aabb.max,
                left_or_first: *triangle_indices.first().unwrap_or(&0),
                right_or_count: triangle_indices.len() as u32,
                is_leaf: 1,
            });
        }
        BvhNode::Internal { aabb, left, right } => {
            let self_idx = out.len();
            // Reserve a slot; we'll fill in child indices once they're known.
            out.push(FlatBvhNode {
                aabb_min: aabb.min,
                aabb_max: aabb.max,
                left_or_first: 0,
                right_or_count: 0,
                is_leaf: 0,
            });

            let left_idx = out.len() as u32;
            flatten_recursive(left, out);

            let right_idx = out.len() as u32;
            flatten_recursive(right, out);

            out[self_idx].left_or_first = left_idx;
            out[self_idx].right_or_count = right_idx;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bvh_build_from_cube() {
        let cube = Mesh::cube(1.0);
        let bvh = Bvh::build(&cube);

        // Collect all triangle indices present in the leaves.
        let mut found = Vec::new();
        collect_leaf_triangles(&bvh.root, &mut found);
        found.sort();
        found.dedup();

        // The cube has 12 triangles; every one should appear.
        assert_eq!(found.len(), 12);
        for i in 0..12u32 {
            assert!(found.contains(&i), "missing triangle {}", i);
        }
    }

    #[test]
    fn bvh_root_aabb_contains_mesh() {
        let cube = Mesh::cube(2.0);
        let bvh = Bvh::build(&cube);
        let mesh_bb = cube.bounding_box();
        let root_bb = bvh.root.aabb();

        // The root AABB must contain every point in the mesh AABB.
        assert!(root_bb.contains_point(mesh_bb.min));
        assert!(root_bb.contains_point(mesh_bb.max));
    }

    #[test]
    fn bvh_leaf_aabbs_contained_in_parent() {
        let sphere = Mesh::uv_sphere(1.0, 16, 8);
        let bvh = Bvh::build(&sphere);
        verify_containment(&bvh.root);
    }

    #[test]
    fn flat_bvh_has_all_nodes() {
        let cube = Mesh::cube(1.0);
        let bvh = Bvh::build(&cube);
        let flat = bvh.flatten();
        assert!(!flat.nodes.is_empty());

        // Count leaf nodes.
        let leaf_count: usize = flat
            .nodes
            .iter()
            .filter(|n| n.is_leaf == 1)
            .map(|n| n.right_or_count as usize)
            .sum();
        assert_eq!(leaf_count, 12); // 12 triangles
    }

    #[test]
    fn bvh_empty_mesh() {
        let empty = Mesh::new("empty");
        let bvh = Bvh::build(&empty);
        match &bvh.root {
            BvhNode::Leaf {
                triangle_indices, ..
            } => assert!(triangle_indices.is_empty()),
            _ => panic!("expected leaf for empty mesh"),
        }
    }

    // -- Helpers -------------------------------------------------------------

    fn collect_leaf_triangles(node: &BvhNode, out: &mut Vec<u32>) {
        match node {
            BvhNode::Leaf {
                triangle_indices, ..
            } => out.extend(triangle_indices),
            BvhNode::Internal { left, right, .. } => {
                collect_leaf_triangles(left, out);
                collect_leaf_triangles(right, out);
            }
        }
    }

    fn verify_containment(node: &BvhNode) {
        if let BvhNode::Internal { aabb, left, right } = node {
            let la = left.aabb();
            let ra = right.aabb();
            // Children must be contained in (or equal to) parent AABB.
            assert!(
                aabb.contains_point(la.min) && aabb.contains_point(la.max),
                "left child AABB not contained in parent"
            );
            assert!(
                aabb.contains_point(ra.min) && aabb.contains_point(ra.max),
                "right child AABB not contained in parent"
            );
            verify_containment(left);
            verify_containment(right);
        }
    }
}
