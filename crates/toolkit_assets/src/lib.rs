//! Asset import/export.
//!
//! Every importer returns the same neutral [`ImportedScene`] (a list of meshes
//! plus the instances that place them), so the rest of the toolkit never needs
//! to know which format a model came from. From there,
//! [`ImportedScene::build_scene`] produces a [`toolkit_scene::Scene`].
//!
//! Supported formats:
//! * **OBJ** — [`import_obj_str`] / [`import_obj_path`] / [`export_obj`].
//! * **glTF 2.0** (`.gltf`/`.glb`) — [`import_gltf_slice`] / [`import_gltf_path`].
//!
//! New formats slot in by producing an [`ImportedScene`]; nothing downstream
//! changes.

pub mod error;
pub mod gltf_io;
pub mod obj;
pub mod scene_import;

pub use error::{AssetError, AssetResult};
pub use gltf_io::{import_gltf_path, import_gltf_slice};
pub use obj::{export_obj, export_obj_path, import_obj_path, import_obj_str};
pub use scene_import::{ImportedScene, MeshInstance};
