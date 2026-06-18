//! Serialization for [`Project`]: human-readable JSON for diffable/debuggable
//! saves, and compact bincode for shipping. Both round-trip the full bundle.

use std::path::Path;

use toolkit_core::{ToolkitError, ToolkitResult};

use crate::project::Project;

fn bincode_err(e: bincode::Error) -> ToolkitError {
    ToolkitError::SerializationError(format!("bincode: {e}"))
}

impl Project {
    /// Serialize to pretty-printed JSON.
    pub fn to_json(&self) -> ToolkitResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parse from a JSON string.
    pub fn from_json(s: &str) -> ToolkitResult<Project> {
        Ok(serde_json::from_str(s)?)
    }

    /// Serialize to compact binary (bincode).
    pub fn to_binary(&self) -> ToolkitResult<Vec<u8>> {
        bincode::serialize(self).map_err(bincode_err)
    }

    /// Deserialize from a binary (bincode) buffer.
    pub fn from_binary(bytes: &[u8]) -> ToolkitResult<Project> {
        bincode::deserialize(bytes).map_err(bincode_err)
    }

    /// Save as JSON to a file.
    pub fn save_json(&self, path: impl AsRef<Path>) -> ToolkitResult<()> {
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    /// Load a JSON project file.
    pub fn load_json(path: impl AsRef<Path>) -> ToolkitResult<Project> {
        let s = std::fs::read_to_string(path)?;
        Self::from_json(&s)
    }

    /// Save as binary to a file.
    pub fn save_binary(&self, path: impl AsRef<Path>) -> ToolkitResult<()> {
        std::fs::write(path, self.to_binary()?)?;
        Ok(())
    }

    /// Load a binary project file.
    pub fn load_binary(path: impl AsRef<Path>) -> ToolkitResult<Project> {
        let bytes = std::fs::read(path)?;
        Self::from_binary(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_geometry::Mesh;
    use toolkit_image::Image;
    use toolkit_render::material::PbrMaterial;
    use toolkit_scene::{NodeData, Transform};

    fn sample_project() -> Project {
        let mut p = Project::new("sample");
        let mesh = p.add_mesh(Mesh::cube(2.0));
        let tex = p.add_texture(Image::filled(2, 2, toolkit_core::LinearRgba::WHITE));
        let mut mat = PbrMaterial::new("painted");
        mat.base_color_texture = Some(tex);
        let mat = p.add_material(mat);
        p.scene.add_node(
            "box",
            Transform::IDENTITY,
            NodeData::Mesh {
                mesh,
                material: Some(mat),
            },
        );
        p
    }

    #[test]
    fn json_roundtrip_preserves_bundle() {
        let p = sample_project();
        let json = p.to_json().unwrap();
        let back = Project::from_json(&json).unwrap();
        assert_eq!(back.metadata.name, "sample");
        assert_eq!(back.mesh_count(), 1);
        assert_eq!(back.material_count(), 1);
        assert_eq!(back.texture_count(), 1);
        assert!(back.validate().is_empty());
    }

    #[test]
    fn binary_roundtrip_preserves_bundle() {
        let p = sample_project();
        let bytes = p.to_binary().unwrap();
        let back = Project::from_binary(&bytes).unwrap();
        assert_eq!(back.scene.len(), 1);
        assert!(back.validate().is_empty());
    }

    #[test]
    fn binary_is_smaller_than_json() {
        let p = sample_project();
        assert!(p.to_binary().unwrap().len() < p.to_json().unwrap().len());
    }

    #[test]
    fn file_roundtrip() {
        let p = sample_project();
        let dir = std::env::temp_dir();
        let path = dir.join(format!("toolkit_project_{}.json", std::process::id()));
        p.save_json(&path).unwrap();
        let back = Project::load_json(&path).unwrap();
        assert_eq!(back.mesh_count(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bad_json_errors() {
        assert!(Project::from_json("{ not valid").is_err());
    }
}
