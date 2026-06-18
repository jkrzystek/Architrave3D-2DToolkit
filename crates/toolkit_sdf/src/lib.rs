//! Signed distance fields (SDF): implicit modeling with smooth booleans.
//!
//! Build a shape as an [`Sdf`] tree of [primitives](primitives) and
//! [CSG combinators](combinators), then [`polygonize`] it into a
//! [`toolkit_geometry::Mesh`] with surface nets. This is the core of an SDF
//! sculpting / implicit-modeling workflow.
//!
//! ```
//! use glam::Vec3;
//! use toolkit_geometry::Aabb;
//! use toolkit_sdf::{Sphere, BoxSdf, smooth_union, polygonize};
//!
//! // A box smoothly blended with a sphere.
//! let shape = smooth_union(
//!     Box::new(BoxSdf { half_extents: Vec3::splat(0.8) }),
//!     Box::new(Sphere { radius: 1.0 }),
//!     0.3,
//! );
//! let mesh = polygonize(shape.as_ref(), &Aabb::new(Vec3::splat(-2.0), Vec3::splat(2.0)), 24);
//! assert!(mesh.triangle_count() > 0);
//! ```

pub mod combinators;
pub mod primitives;
pub mod surface_nets;

pub use combinators::{
    intersection, scale, smin, smooth_intersection, smooth_subtraction, smooth_union, subtraction,
    translate, union, Intersection, Scale, SmoothIntersection, SmoothSubtraction, SmoothUnion,
    Subtraction, Translate, Union,
};
pub use primitives::{
    sd_box, sd_capsule, sd_cylinder, sd_plane, sd_round_box, sd_sphere, sd_torus, sdf_normal,
    BoxSdf, Capsule, Cylinder, Plane, Sdf, Sphere, Torus,
};
pub use surface_nets::polygonize;
