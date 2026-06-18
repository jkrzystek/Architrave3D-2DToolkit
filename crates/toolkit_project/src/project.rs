//! The in-memory project bundle: everything needed to reconstruct a piece of
//! work in one serializable value.

use serde::{Deserialize, Serialize};
use toolkit_core::{MaterialId, MeshId, TextureId};
use toolkit_geometry::Mesh;
use toolkit_image::Image;
use toolkit_render::material::PbrMaterial;
use toolkit_scene::{NodeData, Scene};
use toolkit_units::UnitSystem;

/// Descriptive header for a project: a name, the working unit system, and the
/// tool that produced it. Kept separate so apps can read it cheaply without
/// deserializing the whole bundle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub units: UnitSystem,
    /// Free-form generator/version string, e.g. `"my-app 0.3.1"`.
    pub generator: String,
}

impl ProjectMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            units: UnitSystem::default(),
            generator: String::new(),
        }
    }
}

/// A complete project: scene graph plus the asset tables its nodes reference.
///
/// Assets are stored as `(id, value)` lists rather than maps so the bundle
/// serializes cleanly to both JSON (numeric map keys are not allowed there) and
/// binary formats. UVs ride along inside each [`Mesh`]'s vertices.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub metadata: ProjectMetadata,
    pub scene: Scene,
    meshes: Vec<(MeshId, Mesh)>,
    materials: Vec<(MaterialId, PbrMaterial)>,
    textures: Vec<(TextureId, Image)>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            metadata: ProjectMetadata::new(name),
            scene: Scene::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
        }
    }

    // -- Assets: insertion --------------------------------------------------

    /// Store a mesh under a freshly allocated id.
    pub fn add_mesh(&mut self, mesh: Mesh) -> MeshId {
        let id = MeshId::new();
        self.meshes.push((id, mesh));
        id
    }

    /// Store a mesh under a specific id (e.g. one already referenced by the
    /// scene). Replaces any existing entry with that id.
    pub fn insert_mesh(&mut self, id: MeshId, mesh: Mesh) {
        replace_or_push(&mut self.meshes, id, mesh);
    }

    pub fn add_material(&mut self, material: PbrMaterial) -> MaterialId {
        let id = MaterialId::new();
        self.materials.push((id, material));
        id
    }

    pub fn insert_material(&mut self, id: MaterialId, material: PbrMaterial) {
        replace_or_push(&mut self.materials, id, material);
    }

    pub fn add_texture(&mut self, image: Image) -> TextureId {
        let id = TextureId::new();
        self.textures.push((id, image));
        id
    }

    pub fn insert_texture(&mut self, id: TextureId, image: Image) {
        replace_or_push(&mut self.textures, id, image);
    }

    // -- Assets: lookup -----------------------------------------------------

    pub fn mesh(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.iter().find(|(k, _)| *k == id).map(|(_, m)| m)
    }

    pub fn material(&self, id: MaterialId) -> Option<&PbrMaterial> {
        self.materials.iter().find(|(k, _)| *k == id).map(|(_, m)| m)
    }

    pub fn texture(&self, id: TextureId) -> Option<&Image> {
        self.textures.iter().find(|(k, _)| *k == id).map(|(_, t)| t)
    }

    pub fn meshes(&self) -> impl Iterator<Item = (MeshId, &Mesh)> {
        self.meshes.iter().map(|(k, m)| (*k, m))
    }

    pub fn materials(&self) -> impl Iterator<Item = (MaterialId, &PbrMaterial)> {
        self.materials.iter().map(|(k, m)| (*k, m))
    }

    pub fn textures(&self) -> impl Iterator<Item = (TextureId, &Image)> {
        self.textures.iter().map(|(k, t)| (*k, t))
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn material_count(&self) -> usize {
        self.materials.len()
    }

    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    // -- Integrity ----------------------------------------------------------

    /// Check that every asset id referenced by the scene (and by materials)
    /// resolves to a stored asset. Returns a human-readable list of dangling
    /// references; an empty list means the bundle is self-contained.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        for (key, node) in self.scene.iter() {
            if let NodeData::Mesh { mesh, material } = &node.data {
                if self.mesh(*mesh).is_none() {
                    issues.push(format!(
                        "node {:?} ('{}') references missing mesh {}",
                        key, node.name, mesh
                    ));
                }
                if let Some(mat) = material {
                    if self.material(*mat).is_none() {
                        issues.push(format!(
                            "node {:?} ('{}') references missing material {}",
                            key, node.name, mat
                        ));
                    }
                }
            }
        }

        // Materials may reference textures that must also be embedded.
        for (id, mat) in self.materials() {
            for tex in [
                mat.base_color_texture,
                mat.metallic_roughness_texture,
                mat.normal_texture,
                mat.emissive_texture,
                mat.occlusion_texture,
            ]
            .into_iter()
            .flatten()
            {
                if self.texture(tex).is_none() {
                    issues.push(format!(
                        "material {} ('{}') references missing texture {}",
                        id, mat.name, tex
                    ));
                }
            }
        }

        issues
    }
}

/// Replace the value for `id` if present, otherwise append.
fn replace_or_push<I: PartialEq, V>(list: &mut Vec<(I, V)>, id: I, value: V) {
    if let Some(slot) = list.iter_mut().find(|(k, _)| *k == id) {
        slot.1 = value;
    } else {
        list.push((id, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_scene::Transform;

    #[test]
    fn add_and_look_up_assets() {
        let mut p = Project::new("test");
        let mesh_id = p.add_mesh(Mesh::cube(1.0));
        let mat_id = p.add_material(PbrMaterial::new("steel"));
        assert!(p.mesh(mesh_id).is_some());
        assert_eq!(p.material(mat_id).unwrap().name, "steel");
        assert_eq!(p.mesh_count(), 1);
        assert_eq!(p.material_count(), 1);
    }

    #[test]
    fn insert_with_id_replaces() {
        let mut p = Project::new("test");
        let id = MeshId::from_raw(7);
        p.insert_mesh(id, Mesh::cube(1.0));
        p.insert_mesh(id, Mesh::cube(2.0));
        assert_eq!(p.mesh_count(), 1); // replaced, not duplicated
    }

    #[test]
    fn validate_passes_for_consistent_bundle() {
        let mut p = Project::new("ok");
        let mesh_id = p.add_mesh(Mesh::cube(1.0));
        let mat_id = p.add_material(PbrMaterial::new("m"));
        p.scene.add_node(
            "cube",
            Transform::IDENTITY,
            NodeData::Mesh {
                mesh: mesh_id,
                material: Some(mat_id),
            },
        );
        assert!(p.validate().is_empty());
    }

    #[test]
    fn validate_flags_dangling_mesh() {
        let mut p = Project::new("broken");
        p.scene.add_node(
            "ghost",
            Transform::IDENTITY,
            NodeData::Mesh {
                mesh: MeshId::from_raw(999),
                material: None,
            },
        );
        let issues = p.validate();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("missing mesh"));
    }
}
