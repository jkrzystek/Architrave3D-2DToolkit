use toolkit_geometry::Mesh;
use toolkit_scene::{NodeData, Scene, Transform};

/// One placed instance of an imported mesh.
#[derive(Clone, Debug)]
pub struct MeshInstance {
    pub name: String,
    pub transform: Transform,
    /// Index into [`ImportedScene::meshes`].
    pub mesh_index: usize,
}

/// The neutral result of importing any asset format: a flat list of meshes plus
/// the instances that place them. The application owns the meshes (keyed by
/// their [`MeshId`](toolkit_core::MeshId)); [`build_scene`](ImportedScene::build_scene)
/// produces a [`Scene`] of nodes referencing them.
#[derive(Clone, Debug, Default)]
pub struct ImportedScene {
    pub meshes: Vec<Mesh>,
    pub instances: Vec<MeshInstance>,
}

impl ImportedScene {
    /// Build a [`Scene`] whose nodes reference the imported meshes by id. If
    /// there are no explicit instances, one node is created per mesh.
    pub fn build_scene(&self) -> Scene {
        let mut scene = Scene::new();
        if self.instances.is_empty() {
            for mesh in &self.meshes {
                scene.add_node(
                    mesh.name.clone(),
                    Transform::IDENTITY,
                    NodeData::Mesh {
                        mesh: mesh.id,
                        material: None,
                    },
                );
            }
        } else {
            for inst in &self.instances {
                let mesh_id = self.meshes[inst.mesh_index].id;
                scene.add_node(
                    inst.name.clone(),
                    inst.transform,
                    NodeData::Mesh {
                        mesh: mesh_id,
                        material: None,
                    },
                );
            }
        }
        scene
    }

    pub fn triangle_count(&self) -> usize {
        self.meshes.iter().map(|m| m.triangle_count()).sum()
    }
}
