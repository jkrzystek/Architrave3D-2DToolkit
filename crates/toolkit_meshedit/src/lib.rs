//! Poly-modeling operators on polygon meshes — the bread and butter of "normal"
//! (manual) modeling.
//!
//! Operations work on an [`EditMesh`] (positions + face loops) that round-trips
//! through [`toolkit_topology::HalfEdgeMesh`], so each operator is a small,
//! robust face-list edit rather than fragile half-edge surgery:
//!
//! - [`EditMesh::extrude_face`] / [`extrude_faces`](EditMesh::extrude_faces)
//! - [`EditMesh::inset_face`]
//! - [`EditMesh::bevel_face`] (inset + extrude)
//! - [`EditMesh::bridge_faces`] (connect two open caps into a tube)
//! - [`EditMesh::dissolve_edge`] (merge two faces)
//! - [`EditMesh::fill_hole`] / [`fill_all_holes`](EditMesh::fill_all_holes)
//! - [`EditMesh::loop_cut`] (insert an edge loop across a quad strip)
//!
//! ```
//! use toolkit_meshedit::EditMesh;
//! use toolkit_topology::HalfEdgeMesh;
//! use toolkit_geometry::Mesh;
//!
//! let mut edit = EditMesh::from_halfedge(&HalfEdgeMesh::from_mesh(&Mesh::cube(1.0)));
//! let f = edit.face_count() - 1;
//! edit.extrude_face(f, 0.5);     // pull one face out
//! let renderable = edit.to_mesh("extruded");
//! assert!(renderable.triangle_count() > 12);
//! ```

pub mod edit_mesh;
pub mod loopcut;
pub mod ops;

pub use edit_mesh::EditMesh;
