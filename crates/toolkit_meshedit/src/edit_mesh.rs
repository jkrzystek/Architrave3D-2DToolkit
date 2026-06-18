//! [`EditMesh`]: a plain polygon soup (positions + face vertex-index lists) that
//! editing operations mutate, with conversions to and from
//! [`toolkit_topology::HalfEdgeMesh`].
//!
//! Operating on the polygon list — rather than surgically rewiring half-edge
//! pointers — keeps every operator short and robust: change the faces, then let
//! the half-edge builder reassemble the connectivity. This mirrors
//! `HalfEdgeMesh::rebuild_with_faces`, which exists for exactly this purpose.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use toolkit_geometry::Mesh;
use toolkit_topology::HalfEdgeMesh;

/// A polygon mesh as positions plus per-face vertex-index loops (CCW).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditMesh {
    pub positions: Vec<Vec3>,
    pub faces: Vec<Vec<usize>>,
}

impl EditMesh {
    /// An empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from explicit positions and polygon faces.
    pub fn from_polygons(positions: Vec<Vec3>, faces: Vec<Vec<usize>>) -> Self {
        Self { positions, faces }
    }

    /// Capture the positions and face loops of a half-edge mesh.
    pub fn from_halfedge(he: &HalfEdgeMesh) -> Self {
        Self {
            positions: he.vertices.iter().map(|v| v.position).collect(),
            faces: he.face_list(),
        }
    }

    /// Rebuild a half-edge mesh (recomputing connectivity and normals).
    pub fn to_halfedge(&self) -> HalfEdgeMesh {
        let mut he = HalfEdgeMesh::from_polygons(self.positions.clone(), self.faces.clone());
        he.recompute_normals();
        he
    }

    /// Triangulated, renderable mesh.
    pub fn to_mesh(&self, name: impl Into<String>) -> Mesh {
        self.to_halfedge().to_mesh(name)
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Append a vertex, returning its index.
    pub fn add_vertex(&mut self, p: Vec3) -> usize {
        self.positions.push(p);
        self.positions.len() - 1
    }

    /// Centroid of a face (average of its vertex positions).
    pub fn face_centroid(&self, face: usize) -> Vec3 {
        let vs = &self.faces[face];
        if vs.is_empty() {
            return Vec3::ZERO;
        }
        let sum: Vec3 = vs.iter().map(|&v| self.positions[v]).sum();
        sum / vs.len() as f32
    }

    /// Outward face normal via Newell's method (robust for non-planar polygons,
    /// consistent with CCW winding).
    pub fn face_normal(&self, face: usize) -> Vec3 {
        let vs = &self.faces[face];
        let mut n = Vec3::ZERO;
        let k = vs.len();
        for i in 0..k {
            let a = self.positions[vs[i]];
            let b = self.positions[vs[(i + 1) % k]];
            n.x += (a.y - b.y) * (a.z + b.z);
            n.y += (a.z - b.z) * (a.x + b.x);
            n.z += (a.x - b.x) * (a.y + b.y);
        }
        n.normalize_or_zero()
    }

    /// Remove faces by index (indices need not be sorted). Vertices are kept,
    /// even if they become unreferenced.
    pub fn remove_faces(&mut self, mut indices: Vec<usize>) {
        indices.sort_unstable();
        indices.dedup();
        for &i in indices.iter().rev() {
            if i < self.faces.len() {
                self.faces.remove(i);
            }
        }
    }

    /// Find the boundary loops (ordered vertex rings of holes / open edges).
    ///
    /// A directed face edge with no opposing directed edge is a boundary edge;
    /// chaining those edges head-to-tail yields each hole's loop.
    pub fn boundary_loops(&self) -> Vec<Vec<usize>> {
        use std::collections::HashMap;
        // Directed edges present in the mesh.
        let mut directed: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
        for f in &self.faces {
            let k = f.len();
            for i in 0..k {
                directed.insert((f[i], f[(i + 1) % k]));
            }
        }
        // Boundary edges: present one way but not the other.
        let mut next: HashMap<usize, usize> = HashMap::new();
        for &(a, b) in &directed {
            if !directed.contains(&(b, a)) {
                next.insert(a, b);
            }
        }
        let mut loops = Vec::new();
        let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let starts: Vec<usize> = next.keys().copied().collect();
        for start in starts {
            if visited.contains(&start) {
                continue;
            }
            let mut loop_verts = Vec::new();
            let mut cur = start;
            while let Some(&nxt) = next.get(&cur) {
                if visited.contains(&cur) {
                    break;
                }
                visited.insert(cur);
                loop_verts.push(cur);
                cur = nxt;
                if cur == start {
                    break;
                }
            }
            if loop_verts.len() >= 3 {
                loops.push(loop_verts);
            }
        }
        loops
    }
}

#[cfg(test)]
pub(crate) fn quad_cube() -> EditMesh {
    let p = vec![
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
    ];
    let faces = vec![
        vec![0, 3, 2, 1], // -Z
        vec![4, 5, 6, 7], // +Z
        vec![0, 1, 5, 4], // -Y
        vec![2, 3, 7, 6], // +Y
        vec![1, 2, 6, 5], // +X
        vec![0, 4, 7, 3], // -X
    ];
    EditMesh::from_polygons(p, faces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_points_outward() {
        let cube = quad_cube();
        // +Z face is index 1 with outward normal +Z.
        let n = cube.face_normal(1);
        assert!((n - Vec3::Z).length() < 1e-5, "got {n:?}");
    }

    #[test]
    fn centroid_of_pz_face() {
        let cube = quad_cube();
        assert!((cube.face_centroid(1) - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-6);
    }

    #[test]
    fn roundtrip_through_halfedge() {
        let cube = quad_cube();
        let he = cube.to_halfedge();
        assert_eq!(he.face_count(), 6);
        assert_eq!(he.euler_characteristic(), 2);
        let back = EditMesh::from_halfedge(&he);
        assert_eq!(back.face_count(), 6);
        assert_eq!(back.vertex_count(), 8);
    }

    #[test]
    fn remove_faces_drops_correct_ones() {
        let mut cube = quad_cube();
        cube.remove_faces(vec![0, 1]);
        assert_eq!(cube.face_count(), 4);
    }

    #[test]
    fn open_box_has_one_boundary_loop() {
        let mut cube = quad_cube();
        cube.remove_faces(vec![1]); // open the +Z face
        let loops = cube.boundary_loops();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 4);
    }

    #[test]
    fn serde_roundtrip() {
        let cube = quad_cube();
        let json = serde_json::to_string(&cube).unwrap();
        let back: EditMesh = serde_json::from_str(&json).unwrap();
        assert_eq!(cube, back);
    }
}
