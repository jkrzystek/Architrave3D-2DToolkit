//! Mesh simplification and remeshing.
//!
//! - [`decimate_to`] / [`decimate_ratio`] — QEM (quadric error metric) edge
//!   collapse, which removes triangles while preserving shape (the standard
//!   LOD / sculpt-cleanup simplifier).
//! - [`cluster_remesh`] — fast, robust vertex clustering onto a uniform grid for
//!   unifying resolution and welding duplicate geometry.
//!
//! ```
//! use toolkit_remesh::decimate_ratio;
//! use toolkit_geometry::Mesh;
//!
//! let sphere = Mesh::uv_sphere(1.0, 24, 16);
//! let half = decimate_ratio(&sphere, 0.5);
//! assert!(half.triangle_count() < sphere.triangle_count());
//! ```

pub mod cluster;
pub mod decimate;
pub mod quadric;

pub use cluster::cluster_remesh;
pub use decimate::{decimate_ratio, decimate_to};
pub use quadric::Quadric;
