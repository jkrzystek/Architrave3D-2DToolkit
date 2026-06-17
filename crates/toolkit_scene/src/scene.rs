use glam::Mat4;
use serde::{Deserialize, Serialize};

use crate::node::{NodeData, NodeKey, SceneNode};
use crate::transform::Transform;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Slot {
    generation: u32,
    node: Option<SceneNode>,
}

/// A scene graph: a forest of [`SceneNode`]s stored in a generational arena.
///
/// Nodes reference each other by [`NodeKey`] (stable across insertions and
/// removals). World transforms are computed lazily by walking from the roots in
/// [`update_world_transforms`](Scene::update_world_transforms).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    slots: Vec<Slot>,
    free: Vec<u32>,
    roots: Vec<NodeKey>,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            roots: Vec::new(),
        }
    }

    // -- Creation ------------------------------------------------------------

    /// Add a node at the root level with the given transform and payload.
    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        transform: Transform,
        data: NodeData,
    ) -> NodeKey {
        let node = SceneNode::new(name, transform, data);
        let key = self.insert(node);
        self.roots.push(key);
        key
    }

    /// Add a node as a child of `parent`. Returns `None` if `parent` is invalid.
    pub fn add_child(
        &mut self,
        parent: NodeKey,
        name: impl Into<String>,
        transform: Transform,
        data: NodeData,
    ) -> Option<NodeKey> {
        if !self.is_valid(parent) {
            return None;
        }
        let mut node = SceneNode::new(name, transform, data);
        node.parent = Some(parent);
        let key = self.insert(node);
        self.slots[parent.index as usize]
            .node
            .as_mut()
            .unwrap()
            .children
            .push(key);
        Some(key)
    }

    fn insert(&mut self, node: SceneNode) -> NodeKey {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.node = Some(node);
            NodeKey {
                index,
                generation: slot.generation,
            }
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                node: Some(node),
            });
            NodeKey {
                index,
                generation: 0,
            }
        }
    }

    // -- Access --------------------------------------------------------------

    pub fn is_valid(&self, key: NodeKey) -> bool {
        self.slots
            .get(key.index as usize)
            .map(|s| s.generation == key.generation && s.node.is_some())
            .unwrap_or(false)
    }

    pub fn get(&self, key: NodeKey) -> Option<&SceneNode> {
        let slot = self.slots.get(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.node.as_ref()
    }

    pub fn get_mut(&mut self, key: NodeKey) -> Option<&mut SceneNode> {
        let slot = self.slots.get_mut(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.node.as_mut()
    }

    /// The root-level node keys.
    pub fn roots(&self) -> &[NodeKey] {
        &self.roots
    }

    /// Number of live nodes.
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.node.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over every live node together with its key.
    pub fn iter(&self) -> impl Iterator<Item = (NodeKey, &SceneNode)> {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            slot.node.as_ref().map(|n| {
                (
                    NodeKey {
                        index: i as u32,
                        generation: slot.generation,
                    },
                    n,
                )
            })
        })
    }

    // -- Hierarchy edits -----------------------------------------------------

    /// Re-parent `child` under `new_parent` (or to the root if `None`).
    /// Returns `false` if either key is invalid or the move would create a cycle.
    pub fn set_parent(&mut self, child: NodeKey, new_parent: Option<NodeKey>) -> bool {
        if !self.is_valid(child) {
            return false;
        }
        if let Some(p) = new_parent {
            if !self.is_valid(p) || p == child || self.is_ancestor(child, p) {
                return false;
            }
        }

        // Detach from current parent / roots.
        let old_parent = self.get(child).unwrap().parent;
        match old_parent {
            Some(p) => {
                if let Some(node) = self.get_mut(p) {
                    node.children.retain(|&c| c != child);
                }
            }
            None => self.roots.retain(|&c| c != child),
        }

        // Attach to new parent / roots.
        self.get_mut(child).unwrap().parent = new_parent;
        match new_parent {
            Some(p) => self.get_mut(p).unwrap().children.push(child),
            None => self.roots.push(child),
        }
        true
    }

    /// Returns `true` if `ancestor` is `node` or one of its ancestors.
    pub fn is_ancestor(&self, ancestor: NodeKey, node: NodeKey) -> bool {
        let mut current = Some(node);
        while let Some(c) = current {
            if c == ancestor {
                return true;
            }
            current = self.get(c).and_then(|n| n.parent);
        }
        false
    }

    /// Remove a node and its entire subtree. Returns the number of nodes removed.
    pub fn remove(&mut self, key: NodeKey) -> usize {
        if !self.is_valid(key) {
            return 0;
        }
        // Detach from parent / roots first.
        let parent = self.get(key).unwrap().parent;
        match parent {
            Some(p) => {
                if let Some(node) = self.get_mut(p) {
                    node.children.retain(|&c| c != key);
                }
            }
            None => self.roots.retain(|&c| c != key),
        }
        self.remove_recursive(key)
    }

    fn remove_recursive(&mut self, key: NodeKey) -> usize {
        let children = match self.get(key) {
            Some(n) => n.children.clone(),
            None => return 0,
        };
        let mut count = 1;
        for child in children {
            count += self.remove_recursive(child);
        }
        let slot = &mut self.slots[key.index as usize];
        slot.node = None;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(key.index);
        count
    }

    // -- World transforms ----------------------------------------------------

    /// Recompute every node's cached world matrix by propagating transforms
    /// from the roots down through the hierarchy. Call once per frame after
    /// editing local transforms.
    pub fn update_world_transforms(&mut self) {
        let roots = self.roots.clone();
        for root in roots {
            self.propagate(root, Mat4::IDENTITY);
        }
    }

    fn propagate(&mut self, key: NodeKey, parent_world: Mat4) {
        let (world, children) = {
            let node = match self.get_mut(key) {
                Some(n) => n,
                None => return,
            };
            let world = parent_world * node.transform.to_matrix();
            node.world_matrix = world;
            (world, node.children.clone())
        };
        for child in children {
            self.propagate(child, world);
        }
    }

    /// The world-space transform of a node (valid after
    /// `update_world_transforms`).
    pub fn world_transform(&self, key: NodeKey) -> Option<Transform> {
        self.get(key).map(|n| n.world_transform())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn child_count(scene: &Scene, key: NodeKey) -> usize {
        scene.get(key).unwrap().children().len()
    }

    #[test]
    fn add_and_get_node() {
        let mut scene = Scene::new();
        let k = scene.add_node("root", Transform::IDENTITY, NodeData::Empty);
        assert!(scene.is_valid(k));
        assert_eq!(scene.get(k).unwrap().name, "root");
        assert_eq!(scene.len(), 1);
        assert_eq!(scene.roots().len(), 1);
    }

    #[test]
    fn parent_child_relationship() {
        let mut scene = Scene::new();
        let parent = scene.add_node("parent", Transform::IDENTITY, NodeData::Empty);
        let child = scene
            .add_child(parent, "child", Transform::IDENTITY, NodeData::Empty)
            .unwrap();
        assert_eq!(child_count(&scene, parent), 1);
        assert_eq!(scene.get(child).unwrap().parent(), Some(parent));
        // Child is not a root.
        assert_eq!(scene.roots().len(), 1);
    }

    #[test]
    fn world_transform_accumulates() {
        let mut scene = Scene::new();
        let parent = scene.add_node(
            "p",
            Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)),
            NodeData::Empty,
        );
        let child = scene
            .add_child(
                parent,
                "c",
                Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
                NodeData::Empty,
            )
            .unwrap();
        scene.update_world_transforms();
        let wt = scene.world_transform(child).unwrap();
        assert!((wt.translation - Vec3::new(5.0, 2.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn reparent_moves_subtree() {
        let mut scene = Scene::new();
        let a = scene.add_node("a", Transform::IDENTITY, NodeData::Empty);
        let b = scene.add_node("b", Transform::IDENTITY, NodeData::Empty);
        let c = scene
            .add_child(a, "c", Transform::IDENTITY, NodeData::Empty)
            .unwrap();
        assert!(scene.set_parent(c, Some(b)));
        assert_eq!(child_count(&scene, a), 0);
        assert_eq!(child_count(&scene, b), 1);
        assert_eq!(scene.get(c).unwrap().parent(), Some(b));
    }

    #[test]
    fn reparent_rejects_cycle() {
        let mut scene = Scene::new();
        let a = scene.add_node("a", Transform::IDENTITY, NodeData::Empty);
        let b = scene
            .add_child(a, "b", Transform::IDENTITY, NodeData::Empty)
            .unwrap();
        // Making `a` a child of its own descendant `b` must fail.
        assert!(!scene.set_parent(a, Some(b)));
    }

    #[test]
    fn remove_subtree_frees_nodes() {
        let mut scene = Scene::new();
        let a = scene.add_node("a", Transform::IDENTITY, NodeData::Empty);
        let _b = scene
            .add_child(a, "b", Transform::IDENTITY, NodeData::Empty)
            .unwrap();
        let _c = scene
            .add_child(a, "c", Transform::IDENTITY, NodeData::Empty)
            .unwrap();
        let removed = scene.remove(a);
        assert_eq!(removed, 3);
        assert_eq!(scene.len(), 0);
        assert!(scene.roots().is_empty());
    }

    #[test]
    fn stale_key_after_removal_is_invalid() {
        let mut scene = Scene::new();
        let a = scene.add_node("a", Transform::IDENTITY, NodeData::Empty);
        scene.remove(a);
        // Slot is reused with a bumped generation.
        let b = scene.add_node("b", Transform::IDENTITY, NodeData::Empty);
        assert_eq!(a.index(), b.index());
        assert!(!scene.is_valid(a));
        assert!(scene.is_valid(b));
    }

    #[test]
    fn scene_serializes() {
        let mut scene = Scene::new();
        let p = scene.add_node("p", Transform::IDENTITY, NodeData::Empty);
        scene
            .add_child(p, "c", Transform::IDENTITY, NodeData::Empty)
            .unwrap();
        let json = serde_json::to_string(&scene).unwrap();
        let back: Scene = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
    }
}
