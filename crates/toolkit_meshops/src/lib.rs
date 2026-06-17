//! Mesh utility operations that don't need a half-edge structure.
//!
//! These are the cleanup/processing steps you reach for constantly after
//! generating or importing geometry:
//!
//! * [`weld_vertices`] — merge coincident vertices (de-duplicate soup).
//! * [`recompute_normals`] / [`recompute_tangents`] — regenerate shading attrs.
//! * [`flip_winding`], [`merge`], [`stats`] — orientation, combining, metrics.
//! * [`laplacian_smooth`] — relax vertex positions.
//! * [`decimate_grid`] — simplify by vertex clustering.
//!
//! For topology-aware editing (subdivision, edge flips) use [`toolkit_topology`].
//!
//! ```
//! use toolkit_geometry::Mesh;
//! use toolkit_meshops::weld_vertices;
//!
//! let cube = Mesh::cube(1.0);          // 24 unshared vertices
//! let welded = weld_vertices(&cube, 1e-4);
//! assert_eq!(welded.vertex_count(), 8); // 8 shared corners
//! ```

pub mod attributes;
pub mod decimate;
pub mod ops;
pub mod weld;

pub use attributes::{recompute_normals, recompute_tangents};
pub use decimate::decimate_grid;
pub use ops::{flip_winding, laplacian_smooth, merge, stats, MeshStats};
pub use weld::weld_vertices;
