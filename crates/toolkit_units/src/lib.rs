//! Length units, conversions, and measurement formatting.
//!
//! Lengths are stored canonically in metres ([`Length`]) so arithmetic is
//! unit-agnostic; conversion to a [`LengthUnit`] happens only at construction
//! and display. A [`UnitSystem`] captures a project's display unit and
//! precision, and can parse user input back into a [`Length`].
//!
//! ```
//! use toolkit_units::{Length, LengthUnit, UnitSystem};
//!
//! let sys = UnitSystem::new(LengthUnit::Millimeter, 1);
//! let l = Length::new(1.0, LengthUnit::Inch);
//! assert_eq!(sys.format(l), "25.4 mm");
//!
//! let parsed = sys.parse("2 ft").unwrap();
//! assert!((parsed.in_unit(LengthUnit::Inch) - 24.0).abs() < 1e-9);
//! ```

pub mod length;
pub mod system;

pub use length::{Length, LengthUnit};
pub use system::UnitSystem;
