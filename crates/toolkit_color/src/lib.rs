//! Colour spaces and tools built on [`toolkit_core::LinearRgba`].
//!
//! - [`Hsv`] — hue/saturation/value for pickers and hue rotation.
//! - [`Oklab`] — perceptually uniform space for natural-looking mixing.
//! - [`Gradient`] — multi-stop gradients sampled in linear or OKLab space.
//! - [`Palette`] — named colour sets and evenly distributed category colours.
//!
//! ```
//! use toolkit_color::{Gradient, InterpolationSpace};
//! use toolkit_core::LinearRgba;
//!
//! let g = Gradient::two_stop(LinearRgba::BLACK, LinearRgba::WHITE, InterpolationSpace::Oklab);
//! let swatches = g.ramp(5);
//! assert_eq!(swatches.len(), 5);
//! assert_eq!(swatches[0], LinearRgba::BLACK);
//! ```

pub mod gradient;
pub mod hsv;
pub mod oklab;
pub mod palette;

pub use gradient::{ColorStop, Gradient, InterpolationSpace};
pub use hsv::Hsv;
pub use oklab::Oklab;
pub use palette::Palette;
