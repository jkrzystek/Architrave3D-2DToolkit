//! Turn triangle meshes into volumes — the bridge from surfaces into
//! `toolkit_volume` for remeshing, volume booleans, and sims on arbitrary shapes.
//!
//! [`signed_distance_field`] is the core: per-lattice unsigned distance to the
//! nearest triangle, signed by an inside test (ray-crossing parity).
//! [`solid`] and [`surface_shell`] threshold that field into occupancy and a
//! thin band. [`VoxelizeConfig`] controls grid resolution and padding.
//!
//! ```
//! use toolkit_voxelize::{signed_distance_field, VoxelizeConfig};
//! use toolkit_geometry::Mesh;
//! use glam::Vec3;
//!
//! let sdf = signed_distance_field(&Mesh::cube(2.0), &VoxelizeConfig { resolution: 12, padding: 0.5 });
//! assert!(sdf.sample(Vec3::ZERO) < 0.0); // the cube center is inside
//! ```

pub mod closest;
pub mod voxelize;

pub use closest::closest_point_on_triangle;
pub use voxelize::{signed_distance_field, solid, surface_shell, VoxelizeConfig};
