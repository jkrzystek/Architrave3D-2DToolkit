use glam::Vec3;
use std::collections::HashMap;

use crate::halfedge::{HalfEdgeMesh, VertexId};

/// Errors produced by topology operations with structural preconditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopologyError {
    /// An operation that requires a pure triangle mesh was given a polygon face.
    NonTriangular,
}

impl std::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopologyError::NonTriangular => write!(f, "operation requires a triangle mesh"),
        }
    }
}

impl std::error::Error for TopologyError {}

type EdgeKey = (VertexId, VertexId);

fn edge_key(a: VertexId, b: VertexId) -> EdgeKey {
    (a.min(b), a.max(b))
}

/// Build an edge table from a face list: returns the key->id map, the ordered
/// endpoints per edge, and the faces adjacent to each edge.
fn build_edges(
    faces: &[Vec<VertexId>],
) -> (
    HashMap<EdgeKey, usize>,
    Vec<EdgeKey>,
    Vec<Vec<usize>>,
) {
    let mut index: HashMap<EdgeKey, usize> = HashMap::new();
    let mut endpoints: Vec<EdgeKey> = Vec::new();
    let mut edge_faces: Vec<Vec<usize>> = Vec::new();
    for (fi, f) in faces.iter().enumerate() {
        let k = f.len();
        for i in 0..k {
            let key = edge_key(f[i], f[(i + 1) % k]);
            let id = *index.entry(key).or_insert_with(|| {
                endpoints.push(key);
                edge_faces.push(Vec::new());
                endpoints.len() - 1
            });
            edge_faces[id].push(fi);
        }
    }
    (index, endpoints, edge_faces)
}

impl HalfEdgeMesh {
    /// One round of Catmull-Clark subdivision. Works on arbitrary polygon
    /// meshes and always produces an all-quad mesh that converges to a smooth
    /// limit surface. Boundaries are preserved with the standard crease rules.
    pub fn catmull_clark(&self) -> HalfEdgeMesh {
        let positions: Vec<Vec3> = self.vertices.iter().map(|v| v.position).collect();
        let faces = self.face_list();
        let n_v = positions.len();
        let n_f = faces.len();

        let (edge_index, endpoints, edge_faces) = build_edges(&faces);
        let n_e = endpoints.len();

        // 1. Face points: centroid of each face.
        let face_points: Vec<Vec3> = faces
            .iter()
            .map(|f| f.iter().map(|&v| positions[v]).sum::<Vec3>() / f.len() as f32)
            .collect();

        // 2. Edge points.
        let mut edge_points = vec![Vec3::ZERO; n_e];
        for e in 0..n_e {
            let (a, b) = endpoints[e];
            let mid = (positions[a] + positions[b]) * 0.5;
            if edge_faces[e].len() == 2 {
                let fp = (face_points[edge_faces[e][0]] + face_points[edge_faces[e][1]]) * 0.5;
                edge_points[e] = (mid + fp) * 0.5; // (a + b + f1 + f2) / 4
            } else {
                edge_points[e] = mid; // boundary crease
            }
        }

        // Vertex -> incident faces / edges.
        let mut v_faces: Vec<Vec<usize>> = vec![Vec::new(); n_v];
        let mut v_edges: Vec<Vec<usize>> = vec![Vec::new(); n_v];
        for (fi, f) in faces.iter().enumerate() {
            for &v in f {
                v_faces[v].push(fi);
            }
        }
        for e in 0..n_e {
            let (a, b) = endpoints[e];
            v_edges[a].push(e);
            v_edges[b].push(e);
        }

        // 3. Updated original-vertex positions.
        let mut new_vertices = vec![Vec3::ZERO; n_v];
        for v in 0..n_v {
            let boundary: Vec<usize> = v_edges[v]
                .iter()
                .copied()
                .filter(|&e| edge_faces[e].len() == 1)
                .collect();
            if !boundary.is_empty() {
                // Crease rule: 3/4 P + 1/8 of each of the two boundary neighbours.
                let mut nb = Vec3::ZERO;
                let mut cnt = 0.0;
                for &e in &boundary {
                    let (a, b) = endpoints[e];
                    nb += positions[if a == v { b } else { a }];
                    cnt += 1.0;
                }
                new_vertices[v] = positions[v] * 0.75 + (nb / cnt) * 0.25;
            } else {
                let n = v_faces[v].len() as f32;
                let f_avg =
                    v_faces[v].iter().map(|&fi| face_points[fi]).sum::<Vec3>() / n;
                let r_avg = v_edges[v]
                    .iter()
                    .map(|&e| {
                        let (a, b) = endpoints[e];
                        (positions[a] + positions[b]) * 0.5
                    })
                    .sum::<Vec3>()
                    / v_edges[v].len() as f32;
                new_vertices[v] = (f_avg + r_avg * 2.0 + positions[v] * (n - 3.0)) / n;
            }
        }

        // 4. Assemble the new positions: [vertices | face points | edge points].
        let mut np = new_vertices;
        np.extend_from_slice(&face_points);
        np.extend_from_slice(&edge_points);
        let fp_base = n_v;
        let ep_base = n_v + n_f;

        // 5. Each n-gon becomes n quads.
        let mut new_faces: Vec<Vec<VertexId>> = Vec::new();
        for (fi, f) in faces.iter().enumerate() {
            let k = f.len();
            for i in 0..k {
                let vi = f[i];
                let v_next = f[(i + 1) % k];
                let v_prev = f[(i + k - 1) % k];
                let e_out = edge_index[&edge_key(vi, v_next)];
                let e_in = edge_index[&edge_key(v_prev, vi)];
                new_faces.push(vec![
                    vi,
                    ep_base + e_out,
                    fp_base + fi,
                    ep_base + e_in,
                ]);
            }
        }

        HalfEdgeMesh::from_polygons(np, new_faces)
    }

    /// One round of Loop subdivision. Requires a pure triangle mesh; each
    /// triangle becomes four. Produces smoother results than Catmull-Clark on
    /// triangle meshes.
    pub fn loop_subdivide(&self) -> Result<HalfEdgeMesh, TopologyError> {
        let positions: Vec<Vec3> = self.vertices.iter().map(|v| v.position).collect();
        let faces = self.face_list();
        if faces.iter().any(|f| f.len() != 3) {
            return Err(TopologyError::NonTriangular);
        }
        let n_v = positions.len();
        let (edge_index, endpoints, edge_faces) = build_edges(&faces);
        let n_e = endpoints.len();

        // Vertex one-ring and boundary classification.
        let mut v_edges: Vec<Vec<usize>> = vec![Vec::new(); n_v];
        for e in 0..n_e {
            let (a, b) = endpoints[e];
            v_edges[a].push(e);
            v_edges[b].push(e);
        }

        // Odd (edge) vertices.
        let mut edge_points = vec![Vec3::ZERO; n_e];
        for e in 0..n_e {
            let (a, b) = endpoints[e];
            if edge_faces[e].len() == 2 {
                let c = opposite_vertex(&faces[edge_faces[e][0]], a, b);
                let d = opposite_vertex(&faces[edge_faces[e][1]], a, b);
                edge_points[e] = (positions[a] + positions[b]) * (3.0 / 8.0)
                    + (positions[c] + positions[d]) * (1.0 / 8.0);
            } else {
                edge_points[e] = (positions[a] + positions[b]) * 0.5;
            }
        }

        // Even (original) vertices.
        let mut new_vertices = vec![Vec3::ZERO; n_v];
        for v in 0..n_v {
            let boundary: Vec<usize> = v_edges[v]
                .iter()
                .copied()
                .filter(|&e| edge_faces[e].len() == 1)
                .collect();
            if !boundary.is_empty() {
                let mut nb = Vec3::ZERO;
                for &e in &boundary {
                    let (a, b) = endpoints[e];
                    nb += positions[if a == v { b } else { a }];
                }
                // 3/4 P + 1/8 of each boundary neighbour.
                new_vertices[v] = positions[v] * 0.75 + nb * (1.0 / 8.0);
            } else {
                let n = v_edges[v].len();
                let beta = if n == 3 {
                    3.0 / 16.0
                } else {
                    3.0 / (8.0 * n as f32)
                };
                let mut sum = Vec3::ZERO;
                for &e in &v_edges[v] {
                    let (a, b) = endpoints[e];
                    sum += positions[if a == v { b } else { a }];
                }
                new_vertices[v] = positions[v] * (1.0 - n as f32 * beta) + sum * beta;
            }
        }

        let mut np = new_vertices;
        np.extend_from_slice(&edge_points);
        let ep_base = n_v;

        let mut new_faces: Vec<Vec<VertexId>> = Vec::new();
        for f in &faces {
            let (v0, v1, v2) = (f[0], f[1], f[2]);
            let e01 = ep_base + edge_index[&edge_key(v0, v1)];
            let e12 = ep_base + edge_index[&edge_key(v1, v2)];
            let e20 = ep_base + edge_index[&edge_key(v2, v0)];
            new_faces.push(vec![v0, e01, e20]);
            new_faces.push(vec![v1, e12, e01]);
            new_faces.push(vec![v2, e20, e12]);
            new_faces.push(vec![e01, e12, e20]);
        }

        Ok(HalfEdgeMesh::from_polygons(np, new_faces))
    }
}

fn opposite_vertex(tri: &[VertexId], a: VertexId, b: VertexId) -> VertexId {
    for &v in tri {
        if v != a && v != b {
            return v;
        }
    }
    tri[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halfedge::quad_cube;
    use toolkit_geometry::Mesh;

    #[test]
    fn catmull_clark_quad_counts() {
        let cube = quad_cube();
        let s = cube.catmull_clark();
        // Each of 6 faces splits into 4 quads.
        assert_eq!(s.face_count(), 24);
        // New vertices = original(8) + face points(6) + edge points(12).
        assert_eq!(s.vertex_count(), 26);
        // Still a closed genus-0 surface.
        assert_eq!(s.euler_characteristic(), 2);
    }

    #[test]
    fn catmull_clark_shrinks_toward_sphere() {
        let cube = quad_cube();
        let before = cube
            .vertices
            .iter()
            .map(|v| v.position.length())
            .fold(0.0_f32, f32::max);
        let s = cube.catmull_clark();
        let after = s
            .vertices
            .iter()
            .map(|v| v.position.length())
            .fold(0.0_f32, f32::max);
        // Corners are pulled inward, so the max radius must decrease.
        assert!(after < before, "{after} !< {before}");
    }

    #[test]
    fn catmull_clark_all_faces_are_quads() {
        let s = quad_cube().catmull_clark();
        for f in 0..s.face_count() {
            assert_eq!(s.face_vertices(f).len(), 4);
        }
    }

    #[test]
    fn loop_subdivide_quadruples_triangles() {
        // Tetrahedron: 4 triangles, closed.
        let positions = vec![
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
        ];
        let faces = vec![
            vec![0, 1, 2],
            vec![0, 3, 1],
            vec![0, 2, 3],
            vec![1, 3, 2],
        ];
        let tet = HalfEdgeMesh::from_polygons(positions, faces);
        assert_eq!(tet.euler_characteristic(), 2);
        let s = tet.loop_subdivide().unwrap();
        assert_eq!(s.face_count(), 16); // 4 * 4
        assert_eq!(s.euler_characteristic(), 2);
    }

    #[test]
    fn loop_rejects_quads() {
        let cube = quad_cube();
        assert_eq!(
            cube.loop_subdivide().unwrap_err(),
            TopologyError::NonTriangular
        );
    }

    #[test]
    fn loop_on_triangulated_mesh() {
        let plane = Mesh::plane(2.0, 2.0, 1);
        let he = HalfEdgeMesh::from_mesh(&plane);
        let s = he.loop_subdivide().unwrap();
        // 2 triangles -> 8.
        assert_eq!(s.face_count(), 8);
    }
}
