//! A thin "value at a point" abstraction that unifies noise, SDFs, volumes, and
//! images so they compose in procedural graphs.
//!
//! [`Field`] maps a point to a scalar; [`VectorField`] maps a point to a `Vec3`.
//! Closures implement both for free, so any sampler becomes a field with no
//! wrapper. [`FieldExt`] then layers combinators — `add`, `mul`, `min`/`max`
//! (SDF union/intersection), `clamp`, `remap`, `warp`, `translate` — that build
//! up statically typed pipelines without allocation.
//!
//! ```
//! use toolkit_field::{Field, FieldExt, Sphere};
//! use glam::Vec3;
//!
//! // Union of two sphere SDFs, then push the surface out by 0.1 (a shell).
//! let a = Sphere { center: Vec3::new(-0.5, 0.0, 0.0), radius: 1.0 };
//! let b = Sphere { center: Vec3::new(0.5, 0.0, 0.0), radius: 1.0 };
//! let shape = a.min(b).map(|d| d - 0.1);
//! assert!(shape.sample(Vec3::ZERO) < 0.0);
//! ```

pub mod combinators;
pub mod field;

pub use combinators::FieldExt;
pub use field::{gradient, Constant, Field, Sphere, VectorField};
