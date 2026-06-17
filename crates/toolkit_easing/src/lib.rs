//! Easing, interpolation, and tweening.
//!
//! Pure-math and dependency-light: easing functions ([`Easing`]/[`ease`]),
//! scalar interpolation helpers ([`lerp`], [`inverse_lerp`], [`remap`]), and a
//! value-agnostic time-driven [`Tween`]. The tween yields an eased `0..1`
//! progress you feed into any interpolation, so this crate never needs a math
//! dependency.
//!
//! ```
//! use toolkit_easing::{Tween, Easing, lerp};
//!
//! let mut t = Tween::new(2.0, Easing::CubicInOut);
//! let p = t.update(1.0);            // half a second in
//! let value = lerp(0.0, 100.0, p); // interpolate anything with the progress
//! assert!(value > 0.0 && value < 100.0);
//! ```

pub mod easing;
pub mod tween;

pub use easing::{ease, inverse_lerp, lerp, remap, Easing};
pub use tween::{Repeat, Tween};
