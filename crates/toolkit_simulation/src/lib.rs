pub mod grid;
pub mod fluid;
pub mod erosion;

pub use grid::Grid2D;
pub use fluid::{FluidConfig, FluidSim};
pub use erosion::{ErosionConfig, ErosionSim};
