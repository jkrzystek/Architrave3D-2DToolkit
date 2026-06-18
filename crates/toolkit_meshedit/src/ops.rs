//! Core poly-modeling operators on [`EditMesh`].
//!
//! Each operator mutates the face list in place and adds vertices as needed, so
//! they chain naturally (e.g. `bevel_face` is `inset_face` followed by
//! `extrude_face`).

use glam::Vec3;

use crate::edit_mesh::EditMesh;

impl EditMesh {
    /// Extrude a face along its normal by `distance`, creating side walls.
    ///
    /// The face is replaced by a new "cap" ring of fresh vertices; quad walls
    /// connect the original ring to the cap. Returns the (unchanged) index of the
    /// face, now holding the cap.
    pub fn extrude_face(&mut self, face: usize, distance: f32) -> usize {
        let ring = self.faces[face].clone();
        let offset = self.face_normal(face) * distance;
        let cap: Vec<usize> = ring
            .iter()
            .map(|&v| self.add_vertex(self.positions[v] + offset))
            .collect();
        let k = ring.len();
        for i in 0..k {
            let a = ring[i];
            let b = ring[(i + 1) % k];
            let cb = cap[(i + 1) % k];
            let ca = cap[i];
            // Wall wound so its outward normal faces away from the solid.
            self.faces.push(vec![a, b, cb, ca]);
        }
        self.faces[face] = cap;
        face
    }

    /// Extrude several faces independently (each grows its own walls).
    pub fn extrude_faces(&mut self, faces: &[usize], distance: f32) {
        for &f in faces {
            self.extrude_face(f, distance);
        }
    }

    /// Inset a face: shrink it toward its centroid by `amount` (0 = no change,
    /// 1 = collapse to the centroid), filling the gap with a ring of quads.
    /// Returns the face index, now holding the inner ring.
    pub fn inset_face(&mut self, face: usize, amount: f32) -> usize {
        let ring = self.faces[face].clone();
        let c = self.face_centroid(face);
        let t = amount.clamp(0.0, 1.0);
        let inner: Vec<usize> = ring
            .iter()
            .map(|&v| {
                let p = self.positions[v];
                self.add_vertex(p + (c - p) * t)
            })
            .collect();
        let k = ring.len();
        for i in 0..k {
            let a = ring[i];
            let b = ring[(i + 1) % k];
            let bi = inner[(i + 1) % k];
            let ai = inner[i];
            self.faces.push(vec![a, b, bi, ai]);
        }
        self.faces[face] = inner;
        face
    }

    /// Bevel a face: inset it by `inset`, then extrude the inset cap by `depth`.
    /// A positive `depth` raises the cap, negative cuts inward.
    pub fn bevel_face(&mut self, face: usize, inset: f32, depth: f32) -> usize {
        self.inset_face(face, inset);
        self.extrude_face(face, depth)
    }

    /// Bridge two faces with the same vertex count by connecting their rings
    /// with quad walls, then removing both faces (turning two open caps into a
    /// tube). Vertex `i` of `face_a` connects to vertex `i` of `face_b`; the
    /// second ring is reversed so the bridge does not self-intersect.
    /// Returns `false` (no-op) if the vertex counts differ.
    pub fn bridge_faces(&mut self, face_a: usize, face_b: usize) -> bool {
        let a = self.faces[face_a].clone();
        let b = self.faces[face_b].clone();
        if a.len() != b.len() || a.len() < 3 {
            return false;
        }
        let k = a.len();
        let b_rev: Vec<usize> = b.iter().rev().copied().collect();
        for i in 0..k {
            let a0 = a[i];
            let a1 = a[(i + 1) % k];
            let b1 = b_rev[(i + 1) % k];
            let b0 = b_rev[i];
            self.faces.push(vec![a0, a1, b1, b0]);
        }
        self.remove_faces(vec![face_a, face_b]);
        true
    }

    /// Dissolve the edge shared by two faces, merging them into one polygon.
    /// The two faces must share exactly one edge. Returns `false` if they do
    /// not. On success `face_a` becomes the merged polygon and `face_b` is
    /// removed.
    pub fn dissolve_edge(&mut self, face_a: usize, face_b: usize) -> bool {
        let a = self.faces[face_a].clone();
        let b = self.faces[face_b].clone();
        // Shared edge is u->v in one face and v->u in the other.
        let find_edge = |face: &[usize], u: usize, v: usize| -> Option<usize> {
            let k = face.len();
            (0..k).find(|&i| face[i] == u && face[(i + 1) % k] == v)
        };
        for i in 0..a.len() {
            let u = a[i];
            let v = a[(i + 1) % a.len()];
            if let Some(_j) = find_edge(&b, v, u) {
                // Rotate a to start at v (so it ends at u), b to start at u.
                let ar = rotate_to_start(&a, v); // [v, ..., u]
                let br = rotate_to_start(&b, u); // [u, ..., v]
                let mut merged = Vec::with_capacity(ar.len() + br.len() - 2);
                merged.extend_from_slice(&ar[..ar.len() - 1]); // drop trailing u
                merged.extend_from_slice(&br[..br.len() - 1]); // drop trailing v
                if merged.len() >= 3 {
                    self.faces[face_a] = merged;
                    self.remove_faces(vec![face_b]);
                    return true;
                }
            }
        }
        false
    }

    /// Fill a boundary loop with a single n-gon face (reversed so it faces into
    /// the hole correctly). Returns the new face index.
    pub fn fill_hole(&mut self, loop_verts: &[usize]) -> usize {
        let face: Vec<usize> = loop_verts.iter().rev().copied().collect();
        self.faces.push(face);
        self.faces.len() - 1
    }

    /// Fill every detected boundary loop.
    pub fn fill_all_holes(&mut self) {
        for l in self.boundary_loops() {
            self.fill_hole(&l);
        }
    }
}

/// Rotate `seq` so that it begins at `start` (which must be present).
fn rotate_to_start(seq: &[usize], start: usize) -> Vec<usize> {
    let pos = seq.iter().position(|&v| v == start).unwrap_or(0);
    seq[pos..].iter().chain(&seq[..pos]).copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit_mesh::quad_cube;

    #[test]
    fn extrude_keeps_mesh_closed() {
        let mut cube = quad_cube();
        cube.extrude_face(1, 1.0); // extrude +Z face outward
        let he = cube.to_halfedge();
        // Still closed genus-0: V - E + F = 2.
        assert_eq!(he.euler_characteristic(), 2);
        // Added 4 cap verts and 4 walls; the cap moved out to z=2.
        assert_eq!(cube.vertex_count(), 12);
        let cap = &cube.faces[1];
        for &v in cap {
            assert!((cube.positions[v].z - 2.0).abs() < 1e-5);
        }
    }

    #[test]
    fn extrude_wall_count() {
        let mut cube = quad_cube();
        let before = cube.face_count();
        cube.extrude_face(0, 0.5);
        assert_eq!(cube.face_count(), before + 4); // 4 new walls
    }

    #[test]
    fn inset_shrinks_toward_centroid() {
        let mut cube = quad_cube();
        cube.inset_face(1, 0.5);
        let c = Vec3::new(0.0, 0.0, 1.0);
        for &v in &cube.faces[1] {
            // Inner ring is halfway between original corner and centroid.
            let p = cube.positions[v];
            assert!((p - c).length() < (Vec3::new(1.0, 1.0, 1.0) - c).length());
        }
        assert_eq!(cube.to_halfedge().euler_characteristic(), 2);
    }

    #[test]
    fn bevel_insets_then_extrudes() {
        let mut cube = quad_cube();
        cube.bevel_face(1, 0.3, 0.5);
        // The cap should be raised above z=1.
        for &v in &cube.faces[1] {
            assert!(cube.positions[v].z > 1.0);
        }
        assert_eq!(cube.to_halfedge().euler_characteristic(), 2);
    }

    #[test]
    fn dissolve_merges_two_quads_into_hexagon() {
        // Two quads sharing an edge -> a 6-gon.
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
        ];
        let faces = vec![vec![0, 1, 2, 3], vec![1, 4, 5, 2]];
        let mut m = EditMesh::from_polygons(positions, faces);
        assert!(m.dissolve_edge(0, 1));
        assert_eq!(m.face_count(), 1);
        assert_eq!(m.faces[0].len(), 6);
    }

    #[test]
    fn fill_hole_closes_open_box() {
        let mut cube = quad_cube();
        cube.remove_faces(vec![1]);
        let loops = cube.boundary_loops();
        cube.fill_hole(&loops[0]);
        assert_eq!(cube.to_halfedge().euler_characteristic(), 2);
    }

    #[test]
    fn bridge_two_quads_makes_tube() {
        // Two parallel quads -> bridge into an open-ended tube (4 walls).
        let positions = vec![
            // bottom ring z=0
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            // top ring z=1
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ];
        let faces = vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]];
        let mut m = EditMesh::from_polygons(positions, faces);
        assert!(m.bridge_faces(0, 1));
        assert_eq!(m.face_count(), 4); // 4 walls, both caps removed
    }
}
