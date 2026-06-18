//! Convex geometry: 3D hulls and GJK distance.
//!
//! [`convex_hull`] builds a [`ConvexHull`] (vertices + outward triangles, with
//! `contains` and `to_mesh`) from a point cloud via the incremental algorithm.
//! [`gjk_distance`] / [`hulls_intersect`] measure separation between two convex
//! point sets through their Minkowski difference.
//!
//! ```
//! use glam::Vec3;
//! use toolkit_convex::{convex_hull, gjk_distance};
//!
//! let tetra = vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
//! let hull = convex_hull(&tetra).unwrap();
//! assert_eq!(hull.face_count(), 4);
//!
//! let a = vec![Vec3::ZERO];
//! let b = vec![Vec3::new(3.0, 4.0, 0.0)];
//! assert!((gjk_distance(&a, &b) - 5.0).abs() < 1e-4);
//! ```

pub mod gjk;
pub mod hull;

pub use gjk::{gjk_distance, hulls_intersect};
pub use hull::{convex_hull, ConvexHull};
