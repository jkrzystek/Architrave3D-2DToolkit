#[cfg(feature = "adapter-state")]
pub mod state_adapter;

#[cfg(feature = "adapter-render")]
pub mod render_adapter;

#[cfg(feature = "adapter-geometry")]
pub mod geometry_adapter;

#[cfg(feature = "adapter-graph")]
pub mod graph_adapter;

#[cfg(feature = "adapter-simulation")]
pub mod simulation_adapter;

#[cfg(feature = "adapter-input")]
pub mod input_adapter;

#[cfg(feature = "adapter-ui")]
pub mod ui_adapter;

#[cfg(feature = "adapter-state")]
pub use state_adapter::DocumentBridge;

#[cfg(feature = "adapter-render")]
pub use render_adapter::CameraBridge;

#[cfg(feature = "adapter-geometry")]
pub use geometry_adapter::GeometryBridge;

#[cfg(feature = "adapter-graph")]
pub use graph_adapter::GraphBridge;

#[cfg(feature = "adapter-simulation")]
pub use simulation_adapter::{FluidBridge, ErosionBridge};

#[cfg(feature = "adapter-input")]
pub use input_adapter::InputBridge;

#[cfg(feature = "adapter-ui")]
pub use ui_adapter::UiBridge;
