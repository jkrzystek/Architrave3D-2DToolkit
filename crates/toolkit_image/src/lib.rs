//! A CPU image buffer for texture work and baking.
//!
//! [`Image`] holds tightly packed 8-bit sRGB RGBA pixels and supports
//! [`Image::blit`], [`Image::resize`] (bilinear in linear space),
//! [`Image::sample_bilinear`], and PNG [`Image::encode_png`] /
//! [`Image::decode_png`] (and file `load_png`/`save_png`).
//!
//! ```
//! use toolkit_image::Image;
//! use toolkit_core::LinearRgba;
//!
//! let img = Image::filled(4, 4, LinearRgba::WHITE);
//! let bytes = img.encode_png().unwrap();
//! let back = Image::decode_png(&bytes).unwrap();
//! assert_eq!(img, back);
//! ```

pub mod buffer;
pub mod io;

pub use buffer::Image;
