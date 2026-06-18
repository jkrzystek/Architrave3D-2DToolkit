//! 2D polygon triangulation for procedural shapes, CAD profile faces, and filled
//! text/vector outlines.
//!
//! - [`triangulate`] ear-clips a simple polygon (handles concave shapes and
//!   either winding).
//! - [`triangulate_with_holes`] bridges holes into the outer loop, then clips,
//!   returning a [`Triangulation`] (combined vertices + index triples).
//! - [`triangulate_delaunay`] additionally runs a constrained Lawson flip pass
//!   to push the result toward Delaunay quality (fewer slivers).
//!
//! ```
//! use toolkit_triangulate::triangulate_with_holes;
//! use glam::Vec2;
//!
//! // 4x4 square with a 2x2 hole -> triangles covering area 12.
//! let outer = vec![Vec2::new(0.0,0.0), Vec2::new(4.0,0.0), Vec2::new(4.0,4.0), Vec2::new(0.0,4.0)];
//! let hole  = vec![Vec2::new(1.0,1.0), Vec2::new(1.0,3.0), Vec2::new(3.0,3.0), Vec2::new(3.0,1.0)];
//! let tri = triangulate_with_holes(&outer, &[hole]);
//! assert!(!tri.triangles.is_empty());
//! ```

pub mod delaunay;
pub mod earclip;
pub mod holes;

pub use delaunay::{delaunay_flip, triangulate_delaunay};
pub use earclip::{point_in_triangle, signed_area, triangulate};
pub use holes::{triangulate_with_holes, Triangulation};
