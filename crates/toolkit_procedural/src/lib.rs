//! Procedural generation algorithms for the 3D Rust Toolkit.
//!
//! This crate provides reusable building blocks for procedural content:
//!
//! - [`grid_pack`] — Subdivide a bounding rectangle into a regular grid of
//!   cells with configurable spacing and margins.
//! - [`raymarch_2d`] — Generate 2.5D soft shadow masks by raymarching over a
//!   heightmap.

pub mod grid_pack;
pub mod raymarch_2d;

pub use grid_pack::{GridPacker, GridPackConfig};
pub use raymarch_2d::shadow_raymarch_2d;
