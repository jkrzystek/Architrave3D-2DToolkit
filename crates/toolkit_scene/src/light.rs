use glam::Vec3;
use serde::{Deserialize, Serialize};

/// The kind of light and its kind-specific parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LightKind {
    /// Parallel rays (the sun). Direction comes from the node's transform.
    Directional,
    /// Omnidirectional point light with falloff.
    Point { range: f32 },
    /// Cone-shaped spot light.
    Spot {
        range: f32,
        /// Inner cone half-angle in radians (full intensity inside).
        inner_angle: f32,
        /// Outer cone half-angle in radians (zero intensity outside).
        outer_angle: f32,
    },
}

/// A light source attached to a scene node. The node's transform supplies the
/// light's world position and direction; this struct holds only the radiometric
/// parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub kind: LightKind,
    pub color: Vec3,
    /// Intensity multiplier (candela / lux depending on kind; unitless here).
    pub intensity: f32,
    pub enabled: bool,
}

impl Light {
    pub fn directional(color: Vec3, intensity: f32) -> Self {
        Self {
            kind: LightKind::Directional,
            color,
            intensity,
            enabled: true,
        }
    }

    pub fn point(color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            kind: LightKind::Point { range },
            color,
            intensity,
            enabled: true,
        }
    }

    pub fn spot(
        color: Vec3,
        intensity: f32,
        range: f32,
        inner_angle: f32,
        outer_angle: f32,
    ) -> Self {
        Self {
            kind: LightKind::Spot {
                range,
                inner_angle,
                outer_angle,
            },
            color,
            intensity,
            enabled: true,
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::directional(Vec3::ONE, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_kind() {
        assert!(matches!(
            Light::directional(Vec3::ONE, 1.0).kind,
            LightKind::Directional
        ));
        assert!(matches!(
            Light::point(Vec3::ONE, 1.0, 10.0).kind,
            LightKind::Point { .. }
        ));
        assert!(matches!(
            Light::spot(Vec3::ONE, 1.0, 10.0, 0.3, 0.5).kind,
            LightKind::Spot { .. }
        ));
    }

    #[test]
    fn light_serializes() {
        let l = Light::point(Vec3::new(1.0, 0.5, 0.2), 2.0, 15.0);
        let json = serde_json::to_string(&l).unwrap();
        let back: Light = serde_json::from_str(&json).unwrap();
        assert_eq!(l, back);
    }
}
