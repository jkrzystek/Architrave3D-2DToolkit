//! Joints, skin weights, posing, and linear-blend skinning.
//!
//! A [`Skeleton`] is a flat list of [`Joint`]s linked by parent index; it
//! caches each joint's inverse-bind matrix. A [`Pose`] supplies animated local
//! transforms and produces a skinning-matrix palette via
//! [`Pose::skinning_matrices`]. [`apply_skin`] deforms a mesh with a per-vertex
//! [`Skin`] of [`SkinWeights`] (up to four influences each).
//!
//! ```
//! use glam::{Quat, Vec3};
//! use toolkit_geometry::{Mesh, Vertex};
//! use toolkit_scene::Transform;
//! use toolkit_skeleton::{Joint, Pose, Skeleton, Skin, SkinWeights, apply_skin};
//!
//! let skel = Skeleton::new(vec![Joint::new("root", None, Transform::IDENTITY)]);
//! let mut pose = Pose::rest(&skel);
//! pose.local[0] = Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
//!
//! let mesh = Mesh::with_vertices("m", vec![Vertex::position_only(Vec3::X)], vec![]);
//! let skin = Skin::new(vec![SkinWeights::rigid(0)]);
//! let deformed = apply_skin(&mesh, &skin, &pose.skinning_matrices(&skel));
//! assert!((deformed.vertices[0].position_vec3() - Vec3::Y).length() < 1e-4);
//! ```

pub mod pose;
pub mod skeleton;
pub mod skin;

pub use pose::Pose;
pub use skeleton::{Joint, Skeleton};
pub use skin::{apply_skin, Skin, SkinWeights};
