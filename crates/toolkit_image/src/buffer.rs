//! A CPU-side RGBA8 image buffer.
//!
//! Pixels are stored as sRGB-encoded `[r, g, b, a]` bytes (the texture-friendly
//! layout). Filtering and resampling decode to linear via
//! [`toolkit_core::LinearRgba`] so blends are physically correct.

use serde::{Deserialize, Serialize};
use toolkit_core::LinearRgba;

/// A tightly packed RGBA8 image, 4 bytes per pixel, row-major.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    width: u32,
    height: u32,
    /// `width * height * 4` bytes, row-major, sRGB-encoded RGBA.
    pixels: Vec<u8>,
}

impl Image {
    /// A transparent-black image of the given size.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// An image filled with a single colour.
    pub fn filled(width: u32, height: u32, color: LinearRgba) -> Self {
        let mut img = Self::new(width, height);
        img.fill(color);
        img
    }

    /// Wrap an existing RGBA8 byte buffer. Returns `None` if the length does
    /// not match `width * height * 4`.
    pub fn from_rgba8(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        if pixels.len() == (width as usize) * (height as usize) * 4 {
            Some(Self { width, height, pixels })
        } else {
            None
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Raw sRGB RGBA8 bytes, e.g. for uploading to a texture.
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    #[inline]
    fn index(&self, x: u32, y: u32) -> usize {
        ((y as usize) * (self.width as usize) + (x as usize)) * 4
    }

    /// Sample a raw pixel. Returns `None` if out of bounds.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = self.index(x, y);
        Some([self.pixels[i], self.pixels[i + 1], self.pixels[i + 2], self.pixels[i + 3]])
    }

    /// Write a raw pixel. Out-of-bounds writes are ignored.
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = self.index(x, y);
        self.pixels[i..i + 4].copy_from_slice(&rgba);
    }

    /// Read a pixel as a linear colour.
    pub fn linear_at(&self, x: u32, y: u32) -> Option<LinearRgba> {
        self.pixel(x, y)
            .map(|[r, g, b, a]| LinearRgba::from_srgb_u8(r, g, b, a))
    }

    /// Write a linear colour to a pixel (encoded to sRGB).
    pub fn set_linear(&mut self, x: u32, y: u32, color: LinearRgba) {
        self.set_pixel(x, y, color.to_srgb_u8());
    }

    /// Fill the whole image with one colour.
    pub fn fill(&mut self, color: LinearRgba) {
        let rgba = color.to_srgb_u8();
        for px in self.pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&rgba);
        }
    }

    /// Bilinearly sample the image at normalised UV `[0, 1]` (clamped at edges),
    /// filtering in linear space. Returns transparent black for empty images.
    pub fn sample_bilinear(&self, u: f32, v: f32) -> LinearRgba {
        if self.is_empty() {
            return LinearRgba::TRANSPARENT;
        }
        // Texel-centre mapping.
        let fx = (u * self.width as f32 - 0.5).max(0.0);
        let fy = (v * self.height as f32 - 0.5).max(0.0);
        let x0 = fx.floor() as u32;
        let y0 = fy.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let x0 = x0.min(self.width - 1);
        let y0 = y0.min(self.height - 1);
        let tx = fx - fx.floor();
        let ty = fy - fy.floor();

        let c00 = self.linear_at(x0, y0).unwrap();
        let c10 = self.linear_at(x1, y0).unwrap();
        let c01 = self.linear_at(x0, y1).unwrap();
        let c11 = self.linear_at(x1, y1).unwrap();

        let top = c00.lerp(c10, tx);
        let bottom = c01.lerp(c11, tx);
        top.lerp(bottom, ty)
    }

    /// Copy `src` onto this image with its top-left at `(ox, oy)`, clipping to
    /// bounds. Pixels are copied directly (no alpha blending).
    pub fn blit(&mut self, src: &Image, ox: i32, oy: i32) {
        for sy in 0..src.height {
            let dy = oy + sy as i32;
            if dy < 0 || dy >= self.height as i32 {
                continue;
            }
            for sx in 0..src.width {
                let dx = ox + sx as i32;
                if dx < 0 || dx >= self.width as i32 {
                    continue;
                }
                if let Some(px) = src.pixel(sx, sy) {
                    self.set_pixel(dx as u32, dy as u32, px);
                }
            }
        }
    }

    /// Return a new image resampled to `(new_width, new_height)` using bilinear
    /// filtering in linear space.
    pub fn resize(&self, new_width: u32, new_height: u32) -> Image {
        let mut out = Image::new(new_width, new_height);
        if new_width == 0 || new_height == 0 || self.is_empty() {
            return out;
        }
        for y in 0..new_height {
            let v = (y as f32 + 0.5) / new_height as f32;
            for x in 0..new_width {
                let u = (x as f32 + 0.5) / new_width as f32;
                out.set_linear(x, y, self.sample_bilinear(u, v));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_zeroed() {
        let img = Image::new(4, 3);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 3);
        assert_eq!(img.pixel(0, 0), Some([0, 0, 0, 0]));
    }

    #[test]
    fn set_and_get_pixel() {
        let mut img = Image::new(2, 2);
        img.set_pixel(1, 1, [10, 20, 30, 40]);
        assert_eq!(img.pixel(1, 1), Some([10, 20, 30, 40]));
        assert_eq!(img.pixel(5, 5), None);
    }

    #[test]
    fn fill_sets_every_pixel() {
        let img = Image::filled(3, 3, LinearRgba::WHITE);
        assert_eq!(img.pixel(0, 0), Some([255, 255, 255, 255]));
        assert_eq!(img.pixel(2, 2), Some([255, 255, 255, 255]));
    }

    #[test]
    fn bilinear_constant_image_is_constant() {
        let img = Image::filled(8, 8, LinearRgba::new(0.25, 0.5, 0.75, 1.0));
        let c = img.sample_bilinear(0.5, 0.5);
        assert!((c.r - 0.25).abs() < 2e-2);
        assert!((c.g - 0.5).abs() < 2e-2);
    }

    #[test]
    fn bilinear_interpolates_between_texels() {
        // Two-pixel gradient: black on the left, white on the right.
        let mut img = Image::new(2, 1);
        img.set_linear(0, 0, LinearRgba::BLACK);
        img.set_linear(1, 0, LinearRgba::WHITE);
        // Halfway across should be ~mid-grey in linear space.
        let mid = img.sample_bilinear(0.5, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-2);
    }

    #[test]
    fn blit_clips_to_bounds() {
        let mut dst = Image::new(4, 4);
        let src = Image::filled(2, 2, LinearRgba::WHITE);
        dst.blit(&src, 3, 3); // only one pixel lands inside
        assert_eq!(dst.pixel(3, 3), Some([255, 255, 255, 255]));
        assert_eq!(dst.pixel(0, 0), Some([0, 0, 0, 0]));
    }

    #[test]
    fn resize_changes_dimensions() {
        let img = Image::filled(4, 4, LinearRgba::WHITE);
        let small = img.resize(2, 2);
        assert_eq!(small.width(), 2);
        assert_eq!(small.height(), 2);
        // Constant image stays white.
        assert_eq!(small.pixel(0, 0), Some([255, 255, 255, 255]));
    }

    #[test]
    fn from_rgba8_validates_length() {
        assert!(Image::from_rgba8(2, 2, vec![0; 16]).is_some());
        assert!(Image::from_rgba8(2, 2, vec![0; 10]).is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let img = Image::filled(2, 2, LinearRgba::new(0.1, 0.2, 0.3, 1.0));
        let json = serde_json::to_string(&img).unwrap();
        let back: Image = serde_json::from_str(&json).unwrap();
        assert_eq!(img, back);
    }
}
