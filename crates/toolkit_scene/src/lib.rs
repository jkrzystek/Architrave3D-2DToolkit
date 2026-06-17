//! Scene graph for the toolkit: a transform hierarchy of nodes that can carry
//! meshes, lights, or cameras, plus a selection model.
//!
//! The scene is storage- and protocol-agnostic: it holds references
//! ([`toolkit_core::MeshId`], [`toolkit_core::MaterialId`]) rather than the
//! heavy GPU/CPU data itself, so it composes with any renderer or asset store.

pub mod light;
pub mod node;
pub mod scene;
pub mod selection;
pub mod transform;

pub use light::{Light, LightKind};
pub use node::{NodeData, NodeKey, SceneNode};
pub use scene::Scene;
pub use selection::Selection;
pub use transform::Transform;
