//! Renderer-agnostic transform gizmo.
//!
//! The gizmo is pure interaction logic: given a picking [`Ray`](toolkit_geometry::Ray)
//! it reports which handle is under the cursor ([`Gizmo::hit_test`]) and turns a
//! drag into a [`GizmoDelta`] (translation, rotation, or scale) relative to the
//! drag start. How the handles are *drawn* is left entirely to the application,
//! so this works with any renderer.
//!
//! ```
//! use glam::Vec3;
//! use toolkit_geometry::Ray;
//! use toolkit_gizmo::{Gizmo, GizmoMode};
//!
//! let g = Gizmo::new(Vec3::ZERO, GizmoMode::Translate);
//! let ray = Ray::new(Vec3::new(0.5, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
//! let hit = g.hit_test(&ray, Vec3::NEG_Y);
//! assert!(hit.is_some());
//! ```

pub mod gizmo;
pub mod math;

pub use gizmo::{Gizmo, GizmoAxis, GizmoConfig, GizmoDelta, GizmoHandle, GizmoMode};
pub use math::{
    closest_param_on_line, closest_point_on_line, ray_line_distance, ray_plane_intersection,
};
