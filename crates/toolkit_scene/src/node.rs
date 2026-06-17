use glam::Mat4;
use serde::{Deserialize, Serialize};
use toolkit_core::{MaterialId, MeshId};

use crate::light::Light;
use crate::transform::Transform;

/// A stable handle to a node in a [`Scene`](crate::Scene).
///
/// Handles carry a generation counter so that a key referring to a removed
/// node does not silently alias a newly created node that reused the slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeKey {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl NodeKey {
    /// Reconstruct a key from its raw parts (e.g. when received over a wire
    /// protocol). [`Scene::is_valid`](crate::Scene::is_valid) still guards
    /// against stale or fabricated keys via the generation check.
    pub const fn from_raw_parts(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

/// What a node *is*, beyond its transform. A node is always a transform in the
/// hierarchy; this enum adds optional payload (renderable mesh, light, etc.).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeData {
    /// A pure transform / grouping node with no payload.
    Empty,
    /// A renderable mesh with an optional material override.
    Mesh {
        mesh: MeshId,
        material: Option<MaterialId>,
    },
    /// A light source (parameters here; position/direction from the transform).
    Light(Light),
    /// A camera mount point (the actual projection lives in `toolkit_render`).
    Camera,
}

impl Default for NodeData {
    fn default() -> Self {
        NodeData::Empty
    }
}

/// A single node in the scene graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneNode {
    pub name: String,
    pub transform: Transform,
    pub data: NodeData,
    /// When `false`, this node and its subtree are excluded from rendering.
    pub visible: bool,

    pub(crate) parent: Option<NodeKey>,
    pub(crate) children: Vec<NodeKey>,
    /// Cached world matrix, refreshed by [`Scene::update_world_transforms`].
    pub(crate) world_matrix: Mat4,
}

impl SceneNode {
    pub(crate) fn new(name: impl Into<String>, transform: Transform, data: NodeData) -> Self {
        Self {
            name: name.into(),
            transform,
            data,
            visible: true,
            parent: None,
            children: Vec::new(),
            world_matrix: transform.to_matrix(),
        }
    }

    /// The node's parent, if any.
    pub fn parent(&self) -> Option<NodeKey> {
        self.parent
    }

    /// The node's direct children.
    pub fn children(&self) -> &[NodeKey] {
        &self.children
    }

    /// The cached world matrix (valid after `update_world_transforms`).
    pub fn world_matrix(&self) -> Mat4 {
        self.world_matrix
    }

    /// The cached world transform decomposed into TRS.
    pub fn world_transform(&self) -> Transform {
        Transform::from_matrix(&self.world_matrix)
    }
}
