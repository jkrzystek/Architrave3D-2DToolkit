//! Coherent noise functions for procedural content.
//!
//! All noise is seeded and deterministic. [`Noise`] provides gradient (Perlin),
//! [simplex](Noise::simplex2), and value noise in 2D/3D; [`worley2`]/[`worley3`]
//! provide cellular noise; and [`Fbm`] layers any base noise into fractal detail
//! (including a ridged variant for mountains).
//!
//! ```
//! use toolkit_noise::{Noise, Fbm, NoiseKind};
//!
//! let noise = Noise::new(1234);
//! let height = Fbm::new(NoiseKind::Simplex).sample2(&noise, 3.2, 1.7);
//! assert!(height >= -1.2 && height <= 1.2);
//! ```

pub mod fractal;
pub mod perlin;
pub mod simplex;
pub mod worley;

pub use fractal::{Fbm, NoiseKind};
pub use perlin::Noise;
pub use worley::{worley2, worley2_f2, worley3};
