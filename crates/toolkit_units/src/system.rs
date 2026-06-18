//! A display/measurement preference: which unit to show lengths in, and how
//! many decimals. This is the document-level "what units am I working in"
//! setting an app would persist.

use serde::{Deserialize, Serialize};

use crate::length::{Length, LengthUnit};

/// How lengths are displayed and parsed for a project.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitSystem {
    /// Unit used when formatting bare numbers and as the default for parsing.
    pub display: LengthUnit,
    /// Decimal places used by [`UnitSystem::format`].
    pub precision: usize,
}

impl UnitSystem {
    pub fn new(display: LengthUnit, precision: usize) -> Self {
        Self { display, precision }
    }

    /// Metric default: metres with 3 decimals (millimetre resolution).
    pub fn metric() -> Self {
        Self::new(LengthUnit::Meter, 3)
    }

    /// Imperial default: inches with 3 decimals.
    pub fn imperial() -> Self {
        Self::new(LengthUnit::Inch, 3)
    }

    /// Format a length in the system's display unit and precision.
    pub fn format(&self, length: Length) -> String {
        length.format(self.display, self.precision)
    }

    /// Parse a user string into a [`Length`]. Accepts an explicit unit suffix
    /// (`"12 mm"`, `"3ft"`) or a bare number interpreted in the display unit.
    pub fn parse(&self, input: &str) -> Option<Length> {
        let s = input.trim();
        if s.is_empty() {
            return None;
        }

        // Split the numeric prefix from the trailing unit letters/symbols.
        let split = s
            .find(|c: char| c.is_ascii_alphabetic() || c == '"' || c == '\'')
            .unwrap_or(s.len());
        let (num, unit_str) = s.split_at(split);

        let value: f64 = num.trim().parse().ok()?;
        let unit = if unit_str.trim().is_empty() {
            self.display
        } else {
            LengthUnit::from_abbreviation(unit_str)?
        };
        Some(Length::new(value, unit))
    }
}

impl Default for UnitSystem {
    fn default() -> Self {
        Self::metric()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_in_display_unit() {
        let sys = UnitSystem::new(LengthUnit::Millimeter, 1);
        let l = Length::new(1.0, LengthUnit::Meter);
        assert_eq!(sys.format(l), "1000.0 mm");
    }

    #[test]
    fn parses_explicit_unit() {
        let sys = UnitSystem::metric();
        let l = sys.parse("25.4 mm").unwrap();
        assert!((l.in_unit(LengthUnit::Inch) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn parses_bare_number_in_display_unit() {
        let sys = UnitSystem::new(LengthUnit::Centimeter, 2);
        let l = sys.parse("50").unwrap();
        assert!((l.meters() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn parses_unit_with_no_space() {
        let sys = UnitSystem::metric();
        let l = sys.parse("3ft").unwrap();
        assert!((l.in_unit(LengthUnit::Foot) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_garbage() {
        let sys = UnitSystem::metric();
        assert!(sys.parse("").is_none());
        assert!(sys.parse("abc").is_none());
        assert!(sys.parse("10 furlongs").is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let sys = UnitSystem::imperial();
        let json = serde_json::to_string(&sys).unwrap();
        let back: UnitSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(sys, back);
    }
}
