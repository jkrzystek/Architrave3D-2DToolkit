//! A serializable **project bundle** — the "open / save my work" container.
//!
//! A [`Project`] holds a [`toolkit_scene::Scene`] together with the asset
//! tables its nodes reference: meshes (UVs ride along in their vertices),
//! [`toolkit_render`] PBR materials, and embedded [`toolkit_image`] textures.
//! It saves to pretty JSON ([`Project::to_json`]) or compact bincode
//! ([`Project::to_binary`]), with matching file helpers, and
//! [`Project::validate`] reports any dangling asset references.
//!
//! ```
//! use toolkit_project::Project;
//! use toolkit_geometry::Mesh;
//! use toolkit_scene::{NodeData, Transform};
//!
//! let mut project = Project::new("demo");
//! let mesh = project.add_mesh(Mesh::cube(1.0));
//! project.scene.add_node("cube", Transform::IDENTITY, NodeData::Mesh { mesh, material: None });
//!
//! let json = project.to_json().unwrap();
//! let loaded = Project::from_json(&json).unwrap();
//! assert!(loaded.validate().is_empty());
//! ```

pub mod io;
pub mod project;

pub use project::{Project, ProjectMetadata};
