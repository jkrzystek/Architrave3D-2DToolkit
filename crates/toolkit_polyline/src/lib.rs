//! Polyline operations shared by pen tools, paint strokes, contours, and curve
//! editing — written once over a [`Point`] trait so they serve 2D and 3D alike.
//!
//! - [`length`] / [`resample`] / [`resample_by_spacing`] — arc-length measure
//!   and even respacing (e.g. turning a raw dragged stroke into uniform dabs).
//! - [`smooth_chaikin`] / [`smooth_laplacian`] — corner-cutting and relaxation.
//! - [`simplify`] — Douglas-Peucker point reduction.
//! - [`offset_2d`] — sideways offset / outline of a 2D polyline.
//!
//! ```
//! use toolkit_polyline::{resample, simplify};
//! use glam::Vec2;
//!
//! let stroke = vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)];
//! assert_eq!(resample(&stroke, 5).len(), 5);
//! // The collinear middle point is dropped.
//! assert_eq!(simplify(&stroke, 0.01).len(), 2);
//! ```

pub mod offset;
pub mod ops;
pub mod point;

pub use offset::offset_2d;
pub use ops::{
    length, resample, resample_by_spacing, simplify, smooth_chaikin, smooth_laplacian,
};
pub use point::Point;
