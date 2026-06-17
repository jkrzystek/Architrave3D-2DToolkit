//! Half-edge mesh representation and topology editing.
//!
//! Indexed triangle meshes ([`toolkit_geometry::Mesh`]) are great for rendering
//! but answer no adjacency questions ("which faces share this edge?"). This
//! crate adds a half-edge structure that does, and builds the core editing
//! operations on top of it: Catmull-Clark / Loop subdivision, edge flips,
//! triangulation, and selection sets (the basis for UV seams and modelling
//! tools).
//!
//! ```
//! use toolkit_topology::HalfEdgeMesh;
//! use toolkit_geometry::Mesh;
//!
//! let he = HalfEdgeMesh::from_mesh(&Mesh::plane(2.0, 2.0, 1));
//! let smoother = he.loop_subdivide().unwrap();
//! let renderable = smoother.to_mesh("subdivided");
//! ```

pub mod edit;
pub mod halfedge;
pub mod selection;
pub mod subdivision;

pub use halfedge::{
    EdgeId, FaceId, HalfEdge, HalfEdgeId, HalfEdgeMesh, HeEdge, HeFace, HeVertex, VertexId,
};
pub use selection::{MeshSelection, SelectMode};
pub use subdivision::TopologyError;
