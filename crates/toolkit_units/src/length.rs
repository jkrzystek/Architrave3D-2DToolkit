//! A length stored canonically in metres, convertible to any [`LengthUnit`].

use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A unit of length. Each variant knows how many metres it represents, so all
/// conversions route through metres (the canonical internal unit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LengthUnit {
    Millimeter,
    Centimeter,
    Meter,
    Kilometer,
    Inch,
    Foot,
    Yard,
    Mile,
}

impl LengthUnit {
    /// Metres in one of this unit (the conversion factor to canonical metres).
    pub fn meters_per_unit(self) -> f64 {
        match self {
            Self::Millimeter => 0.001,
            Self::Centimeter => 0.01,
            Self::Meter => 1.0,
            Self::Kilometer => 1000.0,
            // Imperial units defined from the international inch (exactly 25.4 mm).
            Self::Inch => 0.0254,
            Self::Foot => 0.3048,
            Self::Yard => 0.9144,
            Self::Mile => 1609.344,
        }
    }

    /// The conventional abbreviation, e.g. `"mm"`.
    pub fn abbreviation(self) -> &'static str {
        match self {
            Self::Millimeter => "mm",
            Self::Centimeter => "cm",
            Self::Meter => "m",
            Self::Kilometer => "km",
            Self::Inch => "in",
            Self::Foot => "ft",
            Self::Yard => "yd",
            Self::Mile => "mi",
        }
    }

    /// Parse a unit from its abbreviation (case-insensitive).
    pub fn from_abbreviation(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "mm" => Self::Millimeter,
            "cm" => Self::Centimeter,
            "m" => Self::Meter,
            "km" => Self::Kilometer,
            "in" | "\"" => Self::Inch,
            "ft" | "'" => Self::Foot,
            "yd" => Self::Yard,
            "mi" => Self::Mile,
            _ => return None,
        })
    }
}

/// A length, stored as `f64` metres regardless of the unit it was authored in.
/// Storing one canonical unit keeps arithmetic exact and unit-agnostic;
/// conversion happens only at the edges (construction and display).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Length {
    meters: f64,
}

impl Length {
    pub const ZERO: Self = Self { meters: 0.0 };

    /// Construct from a value expressed in `unit`.
    pub fn new(value: f64, unit: LengthUnit) -> Self {
        Self {
            meters: value * unit.meters_per_unit(),
        }
    }

    /// Construct directly from metres.
    pub fn from_meters(meters: f64) -> Self {
        Self { meters }
    }

    /// The raw value in metres.
    pub fn meters(self) -> f64 {
        self.meters
    }

    /// Convert to a value expressed in `unit`.
    pub fn in_unit(self, unit: LengthUnit) -> f64 {
        self.meters / unit.meters_per_unit()
    }

    /// Absolute length.
    pub fn abs(self) -> Self {
        Self {
            meters: self.meters.abs(),
        }
    }

    /// Format with a fixed number of decimal places and the unit abbreviation,
    /// e.g. `"12.50 mm"`.
    pub fn format(self, unit: LengthUnit, precision: usize) -> String {
        format!(
            "{:.*} {}",
            precision,
            self.in_unit(unit),
            unit.abbreviation()
        )
    }
}

impl Add for Length {
    type Output = Length;
    fn add(self, rhs: Length) -> Length {
        Length::from_meters(self.meters + rhs.meters)
    }
}

impl Sub for Length {
    type Output = Length;
    fn sub(self, rhs: Length) -> Length {
        Length::from_meters(self.meters - rhs.meters)
    }
}

impl Neg for Length {
    type Output = Length;
    fn neg(self) -> Length {
        Length::from_meters(-self.meters)
    }
}

impl Mul<f64> for Length {
    type Output = Length;
    fn mul(self, rhs: f64) -> Length {
        Length::from_meters(self.meters * rhs)
    }
}

impl Div<f64> for Length {
    type Output = Length;
    fn div(self, rhs: f64) -> Length {
        Length::from_meters(self.meters / rhs)
    }
}

/// The ratio between two lengths (dimensionless).
impl Div<Length> for Length {
    type Output = f64;
    fn div(self, rhs: Length) -> f64 {
        self.meters / rhs.meters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_units() {
        let l = Length::new(2.0, LengthUnit::Meter);
        assert!((l.in_unit(LengthUnit::Centimeter) - 200.0).abs() < 1e-9);
        assert!((l.in_unit(LengthUnit::Millimeter) - 2000.0).abs() < 1e-9);
    }

    #[test]
    fn inch_is_exactly_25_4_mm() {
        let l = Length::new(1.0, LengthUnit::Inch);
        assert!((l.in_unit(LengthUnit::Millimeter) - 25.4).abs() < 1e-9);
    }

    #[test]
    fn foot_equals_twelve_inches() {
        let foot = Length::new(1.0, LengthUnit::Foot);
        assert!((foot.in_unit(LengthUnit::Inch) - 12.0).abs() < 1e-9);
    }

    #[test]
    fn arithmetic_is_unit_agnostic() {
        let a = Length::new(100.0, LengthUnit::Centimeter); // 1 m
        let b = Length::new(500.0, LengthUnit::Millimeter); // 0.5 m
        let sum = a + b;
        assert!((sum.meters() - 1.5).abs() < 1e-9);
        let ratio = a / b;
        assert!((ratio - 2.0).abs() < 1e-9);
    }

    #[test]
    fn formatting() {
        let l = Length::new(12.5, LengthUnit::Millimeter);
        assert_eq!(l.format(LengthUnit::Millimeter, 2), "12.50 mm");
    }

    #[test]
    fn abbreviation_round_trip() {
        for u in [
            LengthUnit::Millimeter,
            LengthUnit::Meter,
            LengthUnit::Inch,
            LengthUnit::Mile,
        ] {
            assert_eq!(LengthUnit::from_abbreviation(u.abbreviation()), Some(u));
        }
        assert_eq!(LengthUnit::from_abbreviation("furlong"), None);
    }

    #[test]
    fn serde_roundtrip() {
        let l = Length::new(3.0, LengthUnit::Foot);
        let json = serde_json::to_string(&l).unwrap();
        let back: Length = serde_json::from_str(&json).unwrap();
        assert!((l.meters() - back.meters()).abs() < 1e-12);
    }
}
