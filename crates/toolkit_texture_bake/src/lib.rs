//! Texture baking: project a mesh's surface into UV space and write maps.
//!
//! [`rasterize_gbuffer`] turns a UV-mapped [`toolkit_geometry::Mesh`] into a
//! [`GBuffer`] of per-texel position + normal. From there:
//! [`bake_object_normal_map`], [`bake_position_map`], and
//! [`bake_ambient_occlusion`] (hemisphere ray casting against the mesh) write
//! data into a [`toolkit_image::Image`].
//!
//! ```
//! use toolkit_geometry::Mesh;
//! use toolkit_texture_bake::{rasterize_gbuffer, bake_object_normal_map};
//!
//! let plane = Mesh::plane(2.0, 2.0, 1);
//! let gbuffer = rasterize_gbuffer(&plane, 32, 32);
//! assert!(gbuffer.filled_count() > 0);
//!
//! let normal_map = bake_object_normal_map(&gbuffer);
//! assert_eq!(normal_map.width(), 32);
//! ```

pub mod bake;
pub mod rasterize;

pub use bake::{bake_ambient_occlusion, bake_curvature_map, bake_object_normal_map, bake_position_map};
pub use rasterize::{rasterize_gbuffer, GBuffer, GeometrySample};
