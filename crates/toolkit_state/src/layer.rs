use serde::{Deserialize, Serialize};
use toolkit_core::{BlendMode, LayerId, LayerKind};

/// A node in the layer tree.
///
/// Layers can contain children (when `kind == LayerKind::Folder`), forming
/// an arbitrarily nested tree structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerKind,
    pub opacity: f32,
    pub visible: bool,
    pub blend_mode: BlendMode,
    pub locked: bool,
    pub children: Vec<Layer>,
    pub parent_id: Option<LayerId>,
}

impl Layer {
    /// Create a new layer with sensible defaults.
    pub fn new(name: impl Into<String>, kind: LayerKind) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            kind,
            opacity: 1.0,
            visible: true,
            blend_mode: BlendMode::Normal,
            locked: false,
            children: Vec::new(),
            parent_id: None,
        }
    }

    /// Builder: set opacity.
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Builder: set blend mode.
    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Returns `true` if this layer is a folder (can contain children).
    pub fn is_folder(&self) -> bool {
        self.kind == LayerKind::Folder
    }

    /// Add a child layer. Sets the child's `parent_id` to this layer's id.
    pub fn add_child(&mut self, mut child: Layer) {
        child.parent_id = Some(self.id);
        self.children.push(child);
    }

    /// Remove and return the direct child with the given id, or `None`.
    pub fn remove_child(&mut self, id: LayerId) -> Option<Layer> {
        if let Some(pos) = self.children.iter().position(|c| c.id == id) {
            let mut removed = self.children.remove(pos);
            removed.parent_id = None;
            Some(removed)
        } else {
            None
        }
    }

    /// Find a layer by id in this subtree (including self).
    pub fn find(&self, id: LayerId) -> Option<&Layer> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a mutable reference to a layer by id in this subtree (including self).
    pub fn find_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        if self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Depth-first iterator over this layer and all its descendants.
    pub fn depth_first_iter(&self) -> DepthFirstIter<'_> {
        DepthFirstIter {
            stack: vec![self],
        }
    }

    /// Count this layer plus all descendants.
    pub fn subtree_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.subtree_count()).sum::<usize>()
    }
}

/// Depth-first iterator over a layer subtree.
pub struct DepthFirstIter<'a> {
    stack: Vec<&'a Layer>,
}

impl<'a> Iterator for DepthFirstIter<'a> {
    type Item = &'a Layer;

    fn next(&mut self) -> Option<Self::Item> {
        let layer = self.stack.pop()?;
        // Push children in reverse so the first child is visited first.
        for child in layer.children.iter().rev() {
            self.stack.push(child);
        }
        Some(layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_layer_defaults() {
        let layer = Layer::new("Test", LayerKind::Paint);
        assert_eq!(layer.name, "Test");
        assert_eq!(layer.opacity, 1.0);
        assert!(layer.visible);
        assert!(!layer.locked);
        assert_eq!(layer.blend_mode, BlendMode::Normal);
        assert!(layer.children.is_empty());
        assert!(layer.parent_id.is_none());
    }

    #[test]
    fn with_opacity_clamps() {
        let layer = Layer::new("A", LayerKind::Paint).with_opacity(1.5);
        assert_eq!(layer.opacity, 1.0);

        let layer = Layer::new("B", LayerKind::Paint).with_opacity(-0.5);
        assert_eq!(layer.opacity, 0.0);
    }

    #[test]
    fn is_folder() {
        assert!(Layer::new("F", LayerKind::Folder).is_folder());
        assert!(!Layer::new("P", LayerKind::Paint).is_folder());
    }

    #[test]
    fn add_and_remove_child() {
        let mut root = Layer::new("Root", LayerKind::Folder);
        let child = Layer::new("Child", LayerKind::Paint);
        let child_id = child.id;

        root.add_child(child);
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].parent_id, Some(root.id));

        let removed = root.remove_child(child_id).unwrap();
        assert_eq!(removed.id, child_id);
        assert!(removed.parent_id.is_none());
        assert!(root.children.is_empty());
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut root = Layer::new("Root", LayerKind::Folder);
        let bogus_id = LayerId::new();
        assert!(root.remove_child(bogus_id).is_none());
    }

    #[test]
    fn find_in_tree() {
        let mut root = Layer::new("Root", LayerKind::Folder);
        let mut folder = Layer::new("Folder", LayerKind::Folder);
        let deep = Layer::new("Deep", LayerKind::Paint);
        let deep_id = deep.id;

        folder.add_child(deep);
        root.add_child(folder);

        assert!(root.find(deep_id).is_some());
        assert_eq!(root.find(deep_id).unwrap().name, "Deep");
    }

    #[test]
    fn find_mut_in_tree() {
        let mut root = Layer::new("Root", LayerKind::Folder);
        let mut folder = Layer::new("Folder", LayerKind::Folder);
        let child = Layer::new("Child", LayerKind::Paint);
        let child_id = child.id;

        folder.add_child(child);
        root.add_child(folder);

        let found = root.find_mut(child_id).unwrap();
        found.name = "Renamed".into();

        assert_eq!(root.find(child_id).unwrap().name, "Renamed");
    }

    #[test]
    fn find_returns_none_for_missing() {
        let root = Layer::new("Root", LayerKind::Folder);
        let bogus = LayerId::new();
        assert!(root.find(bogus).is_none());
    }

    #[test]
    fn depth_first_iter_order() {
        // Tree:
        //   Root
        //   ├── A
        //   │   ├── A1
        //   │   └── A2
        //   └── B
        let mut root = Layer::new("Root", LayerKind::Folder);
        let mut a = Layer::new("A", LayerKind::Folder);
        let a1 = Layer::new("A1", LayerKind::Paint);
        let a2 = Layer::new("A2", LayerKind::Paint);
        let b = Layer::new("B", LayerKind::Paint);

        a.add_child(a1);
        a.add_child(a2);
        root.add_child(a);
        root.add_child(b);

        let names: Vec<&str> = root
            .depth_first_iter()
            .map(|l| l.name.as_str())
            .collect();
        assert_eq!(names, vec!["Root", "A", "A1", "A2", "B"]);
    }

    #[test]
    fn subtree_count() {
        let mut root = Layer::new("Root", LayerKind::Folder);
        let mut folder = Layer::new("Folder", LayerKind::Folder);
        folder.add_child(Layer::new("C1", LayerKind::Paint));
        folder.add_child(Layer::new("C2", LayerKind::Paint));
        root.add_child(folder);
        root.add_child(Layer::new("Sibling", LayerKind::Paint));
        assert_eq!(root.subtree_count(), 5);
    }

    #[test]
    fn serialization_roundtrip() {
        let mut root = Layer::new("Root", LayerKind::Folder);
        root.add_child(
            Layer::new("Child", LayerKind::Paint)
                .with_opacity(0.7)
                .with_blend_mode(BlendMode::Multiply),
        );

        let json = serde_json::to_string(&root).unwrap();
        let deserialized: Layer = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Root");
        assert_eq!(deserialized.children.len(), 1);
        assert_eq!(deserialized.children[0].name, "Child");
        assert!((deserialized.children[0].opacity - 0.7).abs() < 1e-6);
        assert_eq!(deserialized.children[0].blend_mode, BlendMode::Multiply);
    }
}
