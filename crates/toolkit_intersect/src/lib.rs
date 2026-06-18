//! Closest-point queries, shape overlap tests, and frustum culling.
//!
//! This extends [`toolkit_geometry`]'s ray-centric set with the bounding-shape
//! tests an editor needs for selection, snapping, broad-phase collision, and
//! visibility culling:
//!
//! - [`shapes`] — [`Plane`], [`Sphere`], [`Segment`], [`Capsule`].
//! - [`closest`] — closest point on segment/AABB/plane and between segments.
//! - [`tests_ops`] — sphere/plane/capsule overlap booleans.
//! - [`frustum`] — extract a [`Frustum`] from a view-projection matrix and cull.
//!
//! ```
//! use glam::Vec3;
//! use toolkit_intersect::{Sphere, sphere_sphere};
//!
//! let a = Sphere::new(Vec3::ZERO, 1.0);
//! let b = Sphere::new(Vec3::new(1.5, 0.0, 0.0), 1.0);
//! assert!(sphere_sphere(&a, &b));
//! ```

pub mod closest;
pub mod frustum;
pub mod shapes;
pub mod tests_ops;

pub use closest::{
    closest_point_on_aabb, closest_point_on_plane, closest_point_on_segment,
    closest_points_between_segments,
};
pub use frustum::Frustum;
pub use shapes::{Capsule, Plane, Segment, Sphere};
pub use tests_ops::{
    aabb_plane_side, capsule_capsule, segment_sphere, segment_sphere_gap, sphere_aabb,
    sphere_plane, sphere_sphere,
};
