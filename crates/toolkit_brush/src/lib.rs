//! A geometry-agnostic brush engine: falloff profiles and stroke stamping.
//!
//! A [`Brush`] (radius, strength, [`Falloff`], dab spacing) turns a dragged path
//! into evenly spaced dabs ([`Brush::dab_centers`]) and reports a stroke
//! [`weight`](Brush::stroke_weight) at any point. It never touches geometry
//! itself — callers multiply the returned weights into whatever they edit, so
//! sculpting (displace vertices), mesh painting (blend texels / vertex colors),
//! terrain (raise/lower a heightfield), and weight painting all share one engine.
//!
//! ```
//! use toolkit_brush::{Brush, Falloff};
//! use glam::Vec3;
//!
//! let mut brush = Brush::new(2.0, 1.0);
//! brush.falloff = Falloff::Smooth;
//! let dabs = brush.dab_centers(&[Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)]);
//! // The vertex at the first dab gets full strength; a far one gets nothing.
//! assert!(brush.stroke_weight(&dabs, Vec3::ZERO) > 0.0);
//! assert_eq!(brush.stroke_weight(&dabs, Vec3::new(100.0, 0.0, 0.0)), 0.0);
//! ```

pub mod brush;
pub mod falloff;

pub use brush::Brush;
pub use falloff::Falloff;
