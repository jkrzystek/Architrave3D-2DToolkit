//! PNG encode/decode for [`Image`], plus filesystem helpers.
//!
//! Decoding normalises any source PNG (palette, grayscale, 16-bit, RGB) down to
//! 8-bit RGBA so callers always get the same in-memory layout.

use std::path::Path;

use toolkit_core::{ToolkitError, ToolkitResult};

use crate::buffer::Image;

fn enc_err(e: png::EncodingError) -> ToolkitError {
    ToolkitError::Custom(format!("PNG encode error: {e}"))
}

fn dec_err(e: png::DecodingError) -> ToolkitError {
    ToolkitError::Custom(format!("PNG decode error: {e}"))
}

impl Image {
    /// Encode to PNG bytes (8-bit RGBA).
    pub fn encode_png(&self) -> ToolkitResult<Vec<u8>> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, self.width(), self.height());
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(enc_err)?;
            writer.write_image_data(self.as_bytes()).map_err(enc_err)?;
        }
        Ok(bytes)
    }

    /// Decode PNG bytes into an 8-bit RGBA image.
    pub fn decode_png(bytes: &[u8]) -> ToolkitResult<Image> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        // Normalise odd formats: expand palette/low-bit grayscale to 8-bit,
        // and drop 16-bit channels to 8-bit.
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);

        let mut reader = decoder.read_info().map_err(dec_err)?;
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| ToolkitError::Custom("PNG too large to decode".into()))?;
        let mut buf = vec![0u8; size];
        let info = reader.next_frame(&mut buf).map_err(dec_err)?;
        let data = &buf[..info.buffer_size()];

        let (w, h) = (info.width, info.height);
        let rgba = expand_to_rgba(data, info.color_type, w, h)?;
        Image::from_rgba8(w, h, rgba)
            .ok_or_else(|| ToolkitError::Custom("decoded PNG size mismatch".into()))
    }

    /// Read a PNG file into an image.
    pub fn load_png(path: impl AsRef<Path>) -> ToolkitResult<Image> {
        let bytes = std::fs::read(path)?;
        Self::decode_png(&bytes)
    }

    /// Write the image to a PNG file.
    pub fn save_png(&self, path: impl AsRef<Path>) -> ToolkitResult<()> {
        let bytes = self.encode_png()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

/// Convert a post-transform PNG channel layout to packed RGBA8.
fn expand_to_rgba(
    data: &[u8],
    color: png::ColorType,
    width: u32,
    height: u32,
) -> ToolkitResult<Vec<u8>> {
    let count = (width as usize) * (height as usize);
    let mut out = vec![0u8; count * 4];
    match color {
        png::ColorType::Rgba => return Ok(data.to_vec()),
        png::ColorType::Rgb => {
            for (i, px) in data.chunks_exact(3).enumerate() {
                out[i * 4..i * 4 + 4].copy_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for (i, &g) in data.iter().enumerate() {
                out[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for (i, px) in data.chunks_exact(2).enumerate() {
                out[i * 4..i * 4 + 4].copy_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        png::ColorType::Indexed => {
            return Err(ToolkitError::Custom(
                "indexed PNG not expanded; expected EXPAND transform".into(),
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_core::LinearRgba;

    #[test]
    fn rgba_roundtrips_through_png() {
        let mut img = Image::new(3, 2);
        img.set_pixel(0, 0, [10, 20, 30, 255]);
        img.set_pixel(2, 1, [200, 100, 50, 128]);
        let bytes = img.encode_png().unwrap();
        let back = Image::decode_png(&bytes).unwrap();
        assert_eq!(img, back);
    }

    #[test]
    fn decode_reports_dimensions() {
        let img = Image::filled(5, 4, LinearRgba::WHITE);
        let bytes = img.encode_png().unwrap();
        let back = Image::decode_png(&bytes).unwrap();
        assert_eq!(back.width(), 5);
        assert_eq!(back.height(), 4);
    }

    #[test]
    fn garbage_bytes_error() {
        assert!(Image::decode_png(b"not a png").is_err());
    }

    #[test]
    fn file_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("toolkit_image_test_{}.png", std::process::id()));
        let img = Image::filled(4, 4, LinearRgba::new(0.2, 0.4, 0.6, 1.0));
        img.save_png(&path).unwrap();
        let back = Image::load_png(&path).unwrap();
        assert_eq!(img.width(), back.width());
        assert_eq!(img.pixel(0, 0), back.pixel(0, 0));
        let _ = std::fs::remove_file(&path);
    }
}
