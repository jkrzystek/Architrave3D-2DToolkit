use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toolkit_geometry::{Mesh, Vertex};

pub type HalfEdgeId = usize;
pub type VertexId = usize;
pub type EdgeId = usize;
pub type FaceId = usize;

/// One side of an edge. Boundary half-edges have `face == None` are *not*
/// created; instead a boundary edge simply has `twin == None`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct HalfEdge {
    /// Vertex this half-edge points away from.
    pub origin: VertexId,
    /// The opposing half-edge on the same edge, or `None` on a boundary.
    pub twin: Option<HalfEdgeId>,
    /// Next half-edge around the same face (CCW).
    pub next: HalfEdgeId,
    /// Previous half-edge around the same face.
    pub prev: HalfEdgeId,
    /// The face to the left of this half-edge.
    pub face: FaceId,
    /// The undirected edge this half-edge belongs to.
    pub edge: EdgeId,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct HeVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    /// An arbitrary outgoing half-edge, or `None` for an isolated vertex.
    pub half_edge: Option<HalfEdgeId>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct HeFace {
    /// One half-edge of this face's boundary loop.
    pub half_edge: HalfEdgeId,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct HeEdge {
    /// One of the two half-edges on this edge.
    pub half_edge: HalfEdgeId,
}

/// A half-edge mesh supporting arbitrary polygonal faces.
///
/// Built from indexed triangle [`Mesh`]es or raw polygon soup, it provides the
/// adjacency information (which faces share an edge, the one-ring of a vertex,
/// boundary detection) that indexed meshes lack — the prerequisite for
/// subdivision, UV unwrapping, and topology editing.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HalfEdgeMesh {
    pub vertices: Vec<HeVertex>,
    pub half_edges: Vec<HalfEdge>,
    pub faces: Vec<HeFace>,
    pub edges: Vec<HeEdge>,
}

impl HalfEdgeMesh {
    pub fn new() -> Self {
        Self::default()
    }

    // -- Construction --------------------------------------------------------

    /// Build from a list of vertex positions and polygon faces (each face is an
    /// ordered list of vertex indices, CCW). Normals and UVs default to zero.
    pub fn from_polygons(positions: Vec<Vec3>, faces: Vec<Vec<VertexId>>) -> Self {
        let verts: Vec<HeVertex> = positions
            .into_iter()
            .map(|p| HeVertex {
                position: p,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                half_edge: None,
            })
            .collect();
        Self::assemble(verts, faces)
    }

    /// Build from an indexed triangle [`Mesh`]. Vertex attributes (normal, UV)
    /// are preserved. Shared indices become shared topology; unshared vertices
    /// (as in the primitive cube) yield boundary edges between faces.
    pub fn from_mesh(mesh: &Mesh) -> Self {
        let verts: Vec<HeVertex> = mesh
            .vertices
            .iter()
            .map(|v| HeVertex {
                position: v.position_vec3(),
                normal: v.normal_vec3(),
                uv: v.uv_vec2(),
                half_edge: None,
            })
            .collect();
        let faces: Vec<Vec<VertexId>> = mesh
            .indices
            .chunks_exact(3)
            .map(|t| vec![t[0] as usize, t[1] as usize, t[2] as usize])
            .collect();
        Self::assemble(verts, faces)
    }

    /// Rebuild the connectivity from a new face list while keeping this mesh's
    /// vertex attributes (position, normal, UV). Used by editing operations
    /// that change topology but not vertex data.
    pub fn rebuild_with_faces(&self, faces: Vec<Vec<VertexId>>) -> HalfEdgeMesh {
        let mut verts = self.vertices.clone();
        for v in &mut verts {
            v.half_edge = None;
        }
        Self::assemble(verts, faces)
    }

    fn assemble(mut vertices: Vec<HeVertex>, faces: Vec<Vec<VertexId>>) -> Self {
        let mut half_edges: Vec<HalfEdge> = Vec::new();
        let mut he_faces: Vec<HeFace> = Vec::new();
        let mut edges: Vec<HeEdge> = Vec::new();

        // (origin, dest) -> half-edge, for twin resolution.
        let mut directed: HashMap<(VertexId, VertexId), HalfEdgeId> = HashMap::new();
        // (min, max) -> edge id, so the two directed half-edges share an edge.
        let mut edge_map: HashMap<(VertexId, VertexId), EdgeId> = HashMap::new();

        for face_verts in &faces {
            let k = face_verts.len();
            if k < 3 {
                continue;
            }
            let face_id = he_faces.len();
            let base = half_edges.len();

            for i in 0..k {
                let origin = face_verts[i];
                let dest = face_verts[(i + 1) % k];

                let edge_key = (origin.min(dest), origin.max(dest));
                let edge = *edge_map.entry(edge_key).or_insert_with(|| {
                    let id = edges.len();
                    edges.push(HeEdge { half_edge: base + i });
                    id
                });

                half_edges.push(HalfEdge {
                    origin,
                    twin: None,
                    next: base + (i + 1) % k,
                    prev: base + (i + k - 1) % k,
                    face: face_id,
                    edge,
                });

                let he_id = base + i;
                directed.insert((origin, dest), he_id);
                if vertices[origin].half_edge.is_none() {
                    vertices[origin].half_edge = Some(he_id);
                }
            }
            he_faces.push(HeFace { half_edge: base });
        }

        // Resolve twins.
        for i in 0..half_edges.len() {
            if half_edges[i].twin.is_some() {
                continue;
            }
            let origin = half_edges[i].origin;
            let dest = half_edges[half_edges[i].next].origin;
            if let Some(&opp) = directed.get(&(dest, origin)) {
                half_edges[i].twin = Some(opp);
                half_edges[opp].twin = Some(i);
            }
        }

        Self {
            vertices,
            half_edges,
            faces: he_faces,
            edges,
        }
    }

    // -- Counts --------------------------------------------------------------

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    pub fn half_edge_count(&self) -> usize {
        self.half_edges.len()
    }

    /// Euler characteristic V - E + F. A closed genus-0 surface gives 2.
    pub fn euler_characteristic(&self) -> i64 {
        self.vertex_count() as i64 - self.edge_count() as i64 + self.face_count() as i64
    }

    // -- Queries -------------------------------------------------------------

    /// The ordered vertex indices around a face.
    pub fn face_vertices(&self, face: FaceId) -> Vec<VertexId> {
        let start = self.faces[face].half_edge;
        let mut out = Vec::new();
        let mut h = start;
        loop {
            out.push(self.half_edges[h].origin);
            h = self.half_edges[h].next;
            if h == start {
                break;
            }
        }
        out
    }

    /// Extract every face as an ordered list of vertex indices.
    pub fn face_list(&self) -> Vec<Vec<VertexId>> {
        (0..self.faces.len()).map(|f| self.face_vertices(f)).collect()
    }

    /// `true` if the edge lies on a boundary (has only one adjacent face).
    pub fn is_boundary_edge(&self, edge: EdgeId) -> bool {
        let h = self.edges[edge].half_edge;
        self.half_edges[h].twin.is_none()
    }

    /// `true` if any incident edge of the vertex is a boundary edge.
    pub fn is_boundary_vertex(&self, vertex: VertexId) -> bool {
        self.half_edges
            .iter()
            .any(|he| he.origin == vertex && he.twin.is_none())
            || self
                .half_edges
                .iter()
                .filter(|he| he.twin.is_none())
                .any(|he| self.half_edges[he.next].origin == vertex)
    }

    /// The one-ring of neighbouring vertices (unordered, deduplicated).
    pub fn vertex_neighbors(&self, vertex: VertexId) -> Vec<VertexId> {
        let mut out = Vec::new();
        for he in &self.half_edges {
            if he.origin == vertex {
                let dest = self.half_edges[he.next].origin;
                if !out.contains(&dest) {
                    out.push(dest);
                }
            } else if self.half_edges[he.next].origin == vertex && !out.contains(&he.origin) {
                out.push(he.origin);
            }
        }
        out
    }

    /// Number of edges incident to a vertex.
    pub fn vertex_valence(&self, vertex: VertexId) -> usize {
        self.vertex_neighbors(vertex).len()
    }

    // -- Conversion back to a renderable mesh --------------------------------

    /// Triangulate (fan) every face and produce an indexed [`Mesh`]. Vertex
    /// normals/UVs are taken from the half-edge vertices.
    pub fn to_mesh(&self, name: impl Into<String>) -> Mesh {
        let vertices: Vec<Vertex> = self
            .vertices
            .iter()
            .map(|v| Vertex::new(v.position, v.normal, v.uv))
            .collect();
        let mut indices = Vec::new();
        for f in 0..self.faces.len() {
            let fv = self.face_vertices(f);
            for i in 1..fv.len() - 1 {
                indices.push(fv[0] as u32);
                indices.push(fv[i] as u32);
                indices.push(fv[i + 1] as u32);
            }
        }
        Mesh::with_vertices(name, vertices, indices)
    }

    /// Recompute per-vertex normals as the area-weighted average of incident
    /// face normals.
    pub fn recompute_normals(&mut self) {
        let mut acc = vec![Vec3::ZERO; self.vertices.len()];
        for f in 0..self.faces.len() {
            let fv = self.face_vertices(f);
            if fv.len() < 3 {
                continue;
            }
            let p0 = self.vertices[fv[0]].position;
            let p1 = self.vertices[fv[1]].position;
            let p2 = self.vertices[fv[2]].position;
            // Cross product magnitude is proportional to area, giving weighting.
            let n = (p1 - p0).cross(p2 - p0);
            for &v in &fv {
                acc[v] += n;
            }
        }
        for (v, n) in self.vertices.iter_mut().zip(acc) {
            v.normal = n.normalize_or_zero();
        }
    }
}

#[cfg(test)]
pub(crate) fn quad_cube() -> HalfEdgeMesh {
    // 8 corners, 6 quad faces — a topologically closed cube.
    let p = vec![
        Vec3::new(-1.0, -1.0, -1.0), // 0
        Vec3::new(1.0, -1.0, -1.0),  // 1
        Vec3::new(1.0, 1.0, -1.0),   // 2
        Vec3::new(-1.0, 1.0, -1.0),  // 3
        Vec3::new(-1.0, -1.0, 1.0),  // 4
        Vec3::new(1.0, -1.0, 1.0),   // 5
        Vec3::new(1.0, 1.0, 1.0),    // 6
        Vec3::new(-1.0, 1.0, 1.0),   // 7
    ];
    let faces = vec![
        vec![0, 3, 2, 1], // -Z
        vec![4, 5, 6, 7], // +Z
        vec![0, 1, 5, 4], // -Y
        vec![2, 3, 7, 6], // +Y
        vec![1, 2, 6, 5], // +X
        vec![0, 4, 7, 3], // -X
    ];
    HalfEdgeMesh::from_polygons(p, faces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_topology_counts() {
        let cube = quad_cube();
        assert_eq!(cube.vertex_count(), 8);
        assert_eq!(cube.face_count(), 6);
        assert_eq!(cube.edge_count(), 12);
        // Closed genus-0 surface: V - E + F = 2.
        assert_eq!(cube.euler_characteristic(), 2);
    }

    #[test]
    fn cube_has_no_boundary() {
        let cube = quad_cube();
        for e in 0..cube.edge_count() {
            assert!(!cube.is_boundary_edge(e), "edge {e} unexpectedly boundary");
        }
        for v in 0..cube.vertex_count() {
            assert!(!cube.is_boundary_vertex(v));
        }
    }

    #[test]
    fn cube_vertex_valence_is_three() {
        let cube = quad_cube();
        for v in 0..cube.vertex_count() {
            assert_eq!(cube.vertex_valence(v), 3);
        }
    }

    #[test]
    fn twins_are_symmetric() {
        let cube = quad_cube();
        for (i, he) in cube.half_edges.iter().enumerate() {
            if let Some(t) = he.twin {
                assert_eq!(cube.half_edges[t].twin, Some(i));
            }
        }
    }

    #[test]
    fn from_mesh_plane_has_boundary() {
        let plane = Mesh::plane(2.0, 2.0, 2);
        let he = HalfEdgeMesh::from_mesh(&plane);
        // A plane is an open surface; its outer ring must be boundary edges.
        let boundary = (0..he.edge_count())
            .filter(|&e| he.is_boundary_edge(e))
            .count();
        assert!(boundary > 0, "plane should have boundary edges");
    }

    #[test]
    fn roundtrip_to_mesh_triangulates_quads() {
        let cube = quad_cube();
        let mesh = cube.to_mesh("cube");
        // 6 quads -> 12 triangles.
        assert_eq!(mesh.triangle_count(), 12);
        assert_eq!(mesh.vertex_count(), 8);
    }

    #[test]
    fn recompute_normals_point_outward() {
        let mut cube = quad_cube();
        cube.recompute_normals();
        for v in &cube.vertices {
            // On a centered cube each corner normal roughly points away from origin.
            assert!(v.normal.dot(v.position) > 0.0);
        }
    }

    #[test]
    fn serializes() {
        let cube = quad_cube();
        let json = serde_json::to_string(&cube).unwrap();
        let back: HalfEdgeMesh = serde_json::from_str(&json).unwrap();
        assert_eq!(back.face_count(), 6);
    }
}
