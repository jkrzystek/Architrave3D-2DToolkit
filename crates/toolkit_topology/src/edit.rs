use crate::halfedge::{EdgeId, HalfEdgeMesh};

impl HalfEdgeMesh {
    /// Flip an interior edge shared by two triangles (the classic 2-2 swap used
    /// in remeshing and Delaunay refinement). Returns a new mesh, or `None` if
    /// the edge is on a boundary or either adjacent face is not a triangle.
    pub fn flip_edge(&self, edge: EdgeId) -> Option<HalfEdgeMesh> {
        let h = self.edges[edge].half_edge;
        let twin = self.half_edges[h].twin?;
        let f1 = self.half_edges[h].face;
        let f2 = self.half_edges[twin].face;

        let x = self.half_edges[h].origin;
        let y = self.half_edges[self.half_edges[h].next].origin;

        let fv1 = self.face_vertices(f1);
        let fv2 = self.face_vertices(f2);
        if fv1.len() != 3 || fv2.len() != 3 {
            return None;
        }

        // The vertices opposite the shared edge in each triangle.
        let c = *fv1.iter().find(|&&v| v != x && v != y)?;
        let d = *fv2.iter().find(|&&v| v != x && v != y)?;

        // The quad (x, d, y, c) is re-triangulated along the c-d diagonal.
        let mut faces = self.face_list();
        faces[f1] = vec![x, d, c];
        faces[f2] = vec![d, y, c];
        Some(self.rebuild_with_faces(faces))
    }

    /// Return a copy with every face fan-triangulated. Polygon meshes become
    /// triangle meshes suitable for Loop subdivision or GPU upload.
    pub fn triangulated(&self) -> HalfEdgeMesh {
        let mut faces = Vec::new();
        for f in 0..self.face_count() {
            let fv = self.face_vertices(f);
            for i in 1..fv.len() - 1 {
                faces.push(vec![fv[0], fv[i], fv[i + 1]]);
            }
        }
        self.rebuild_with_faces(faces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    /// Two triangles sharing edge (1,2) forming a square in the XY plane.
    fn two_triangles() -> HalfEdgeMesh {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(0.0, 1.0, 0.0), // 2
            Vec3::new(1.0, 1.0, 0.0), // 3
        ];
        // Diagonal is edge (1,2).
        let faces = vec![vec![0, 1, 2], vec![1, 3, 2]];
        HalfEdgeMesh::from_polygons(positions, faces)
    }

    fn find_interior_edge(mesh: &HalfEdgeMesh) -> usize {
        (0..mesh.edge_count())
            .find(|&e| !mesh.is_boundary_edge(e))
            .expect("expected an interior edge")
    }

    #[test]
    fn flip_changes_diagonal() {
        let mesh = two_triangles();
        let edge = find_interior_edge(&mesh);
        let flipped = mesh.flip_edge(edge).expect("flip should succeed");
        assert_eq!(flipped.face_count(), 2);
        // The new interior edge must connect 0 and 3 (the other diagonal).
        let new_edge = find_interior_edge(&flipped);
        let h = flipped.edges[new_edge].half_edge;
        let a = flipped.half_edges[h].origin;
        let b = flipped.half_edges[flipped.half_edges[h].next].origin;
        let pair = (a.min(b), a.max(b));
        assert_eq!(pair, (0, 3));
    }

    #[test]
    fn flip_boundary_edge_fails() {
        let mesh = two_triangles();
        let boundary = (0..mesh.edge_count())
            .find(|&e| mesh.is_boundary_edge(e))
            .unwrap();
        assert!(mesh.flip_edge(boundary).is_none());
    }

    #[test]
    fn triangulate_quad_cube() {
        let cube = crate::halfedge::quad_cube();
        let tri = cube.triangulated();
        assert_eq!(tri.face_count(), 12);
        for f in 0..tri.face_count() {
            assert_eq!(tri.face_vertices(f).len(), 3);
        }
    }
}
