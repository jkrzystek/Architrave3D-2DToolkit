use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LinearRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl LinearRgba {
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_srgb(sr: f32, sg: f32, sb: f32, a: f32) -> Self {
        Self {
            r: srgb_to_linear(sr),
            g: srgb_to_linear(sg),
            b: srgb_to_linear(sb),
            a,
        }
    }

    pub fn from_srgb_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::from_srgb(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    pub fn to_srgb(self) -> [f32; 4] {
        [
            linear_to_srgb(self.r),
            linear_to_srgb(self.g),
            linear_to_srgb(self.b),
            self.a,
        ]
    }

    pub fn to_srgb_u8(self) -> [u8; 4] {
        let srgb = self.to_srgb();
        [
            (srgb[0] * 255.0).round().clamp(0.0, 255.0) as u8,
            (srgb[1] * 255.0).round().clamp(0.0, 255.0) as u8,
            (srgb[2] * 255.0).round().clamp(0.0, 255.0) as u8,
            (srgb[3] * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    pub fn premultiplied(self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }

    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
}

impl Default for LinearRgba {
    fn default() -> Self {
        Self::BLACK
    }
}

impl From<[f32; 4]> for LinearRgba {
    fn from(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<LinearRgba> for [f32; 4] {
    fn from(c: LinearRgba) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_linear_roundtrip() {
        for i in 0..=255 {
            let srgb = i as f32 / 255.0;
            let linear = srgb_to_linear(srgb);
            let back = linear_to_srgb(linear);
            assert!((srgb - back).abs() < 1e-5, "roundtrip failed for {srgb}");
        }
    }

    #[test]
    fn black_is_zero() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert_eq!(linear_to_srgb(0.0), 0.0);
    }

    #[test]
    fn white_is_one() {
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        assert!((linear_to_srgb(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn from_srgb_u8() {
        let c = LinearRgba::from_srgb_u8(128, 128, 128, 255);
        let back = c.to_srgb_u8();
        assert_eq!(back[0], 128);
        assert_eq!(back[1], 128);
        assert_eq!(back[2], 128);
        assert_eq!(back[3], 255);
    }

    #[test]
    fn lerp_endpoints() {
        let a = LinearRgba::BLACK;
        let b = LinearRgba::WHITE;
        let at0 = a.lerp(b, 0.0);
        let at1 = a.lerp(b, 1.0);
        assert_eq!(at0, a);
        assert_eq!(at1.r, b.r);
    }

    #[test]
    fn luminance_white() {
        let lum = LinearRgba::WHITE.luminance();
        assert!((lum - 1.0).abs() < 1e-4);
    }

    #[test]
    fn premultiplied_alpha() {
        let c = LinearRgba::new(1.0, 0.5, 0.0, 0.5);
        let pm = c.premultiplied();
        assert!((pm.r - 0.5).abs() < 1e-6);
        assert!((pm.g - 0.25).abs() < 1e-6);
        assert_eq!(pm.a, 0.5);
    }
}
