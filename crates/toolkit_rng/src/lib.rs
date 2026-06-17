//! Deterministic seeded random numbers.
//!
//! [`Rng`] is a PCG32 generator: small, fast, and reproducible across platforms
//! for a given seed — the property procedural generation depends on. It offers
//! uniform integers/floats, normal samples, shuffling, geometric sampling
//! (disk/sphere/hemisphere), and [`poisson_disk_2d`] blue-noise point sets.
//!
//! ```
//! use toolkit_rng::Rng;
//!
//! let mut a = Rng::seed_from_u64(123);
//! let mut b = Rng::seed_from_u64(123);
//! assert_eq!(a.next_u32(), b.next_u32()); // same seed -> same stream
//!
//! let dir = a.unit_vec3();
//! assert!((dir.length() - 1.0).abs() < 1e-4);
//! ```

pub mod rng;
pub mod sampling;

pub use rng::Rng;
pub use sampling::poisson_disk_2d;
