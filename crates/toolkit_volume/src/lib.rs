//! Dense 3D grids of scalars or vectors, placed and sampled in world space.
//!
//! A [`Volume<T>`] stores `T` at the lattice points of a regular grid, located
//! by an `origin` and `cell_size`. Any [`VolumeSample`] cell type (`f32`,
//! `Vec2/3/4`) can be trilinearly [`sample`](Volume::sample)d at a continuous
//! world point, [`resample`](Volume::resample)d to a different resolution, and —
//! for scalar fields — differentiated with [`gradient`](Volume::gradient). This
//! is the 3D counterpart to `toolkit_simulation::Grid2D`, used by voxel
//! sculpting, 3D fluids/erosion, and SDF/density baking.
//!
//! ```
//! use toolkit_volume::Volume;
//! use glam::Vec3;
//!
//! // A scalar ramp along x, then sample halfway between two lattice points.
//! let v = Volume::from_fn([2, 1, 1], Vec3::ZERO, Vec3::ONE, |[x, _, _]| x as f32);
//! assert!((v.sample(Vec3::new(0.5, 0.0, 0.0)) - 0.5).abs() < 1e-6);
//! ```

pub mod sample;
pub mod volume;

pub use sample::VolumeSample;
pub use volume::Volume;
