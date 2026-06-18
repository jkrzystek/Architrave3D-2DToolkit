//! Generate meshes from profiles and paths — the CAD/procedural surfacing kit.
//!
//! - [`extrude`] a closed 2D profile along +Z (with optional end caps).
//! - [`revolve`] a `(radius, height)` profile around the Y axis.
//! - [`loft`] a surface through a sequence of cross-sections.
//! - [`sweep`] a 2D profile along a 3D path using rotation-minimizing frames.
//!
//! All return a renderable [`toolkit_geometry::Mesh`] with smooth normals.
//!
//! ```
//! use toolkit_surfacing::extrude;
//! use glam::Vec2;
//!
//! let square = vec![
//!     Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0),
//!     Vec2::new(1.0, 1.0),   Vec2::new(-1.0, 1.0),
//! ];
//! let box_mesh = extrude(&square, 2.0, true);
//! assert_eq!(box_mesh.triangle_count(), 12); // a closed box
//! ```

pub mod build;
pub mod ops;

pub use build::{finish_mesh, surface_from_grid};
pub use ops::{extrude, loft, revolve, sweep};
