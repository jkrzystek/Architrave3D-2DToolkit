//! Spatial acceleration structures for neighbour and range queries.
//!
//! * [`SpatialHashGrid`] — dynamic, insert/clear per frame; best when points
//!   move and queries use a consistent radius (particles, broad-phase).
//! * [`KdTree`] — static, build once; best for nearest-neighbour and radius
//!   queries over a fixed set (point clouds, sampling).
//! * [`Octree`] — hierarchical box/range queries over uneven distributions.
//!
//! ```
//! use glam::Vec3;
//! use toolkit_spatial::KdTree;
//!
//! let pts = vec![Vec3::new(3.0, 0.0, 0.0), Vec3::new(0.1, 0.0, 0.0), Vec3::new(-5.0, 0.0, 0.0)];
//! let tree = KdTree::build(&pts);
//! assert_eq!(tree.nearest(Vec3::ZERO), Some(1));
//! ```

pub mod grid;
pub mod kdtree;
pub mod octree;
pub mod poisson;

pub use grid::SpatialHashGrid;
pub use kdtree::KdTree;
pub use octree::Octree;
pub use poisson::{poisson_disk_surface_sample, SurfaceSample};
