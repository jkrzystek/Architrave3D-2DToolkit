use serde::{Deserialize, Serialize};
use toolkit_core::{LayerId, LayerKind};

use crate::layer::Layer;

/// The top-level document model.
///
/// Contains a root folder layer that holds the layer tree, plus metadata
/// such as canvas dimensions and the currently active layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// The root layer (always a Folder).
    pub root_layer: Layer,
    /// The id of the currently active (selected) layer, if any.
    pub active_layer_id: Option<LayerId>,
    /// Canvas width in pixels.
    pub width: u32,
    /// Canvas height in pixels.
    pub height: u32,
    /// Document name.
    pub name: String,
    /// Whether the document has unsaved changes.
    #[serde(skip)]
    pub dirty: bool,
}

impl Document {
    /// Create a new document with a root folder layer.
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            root_layer: Layer::new("Root", LayerKind::Folder),
            active_layer_id: None,
            width,
            height,
            name: name.into(),
            dirty: false,
        }
    }

    /// Add a new layer as a child of `parent_id` (or the root if `None`).
    ///
    /// Returns the new layer's id.
    pub fn add_layer(
        &mut self,
        name: impl Into<String>,
        kind: LayerKind,
        parent_id: Option<LayerId>,
    ) -> LayerId {
        let layer = Layer::new(name, kind);
        let id = layer.id;

        // Resolve the parent: if a parent_id is given and exists, use it;
        // otherwise fall back to the root.
        let add_to_root = match parent_id {
            Some(pid) => self.root_layer.find(pid).is_none(),
            None => true,
        };

        if add_to_root {
            self.root_layer.add_child(layer);
        } else {
            // Safe: we just confirmed the id exists.
            self.root_layer
                .find_mut(parent_id.unwrap())
                .unwrap()
                .add_child(layer);
        }

        self.dirty = true;
        id
    }

    /// Remove a layer by id from anywhere in the tree.
    ///
    /// Returns the removed layer, or `None` if not found.
    /// Cannot remove the root layer.
    pub fn remove_layer(&mut self, id: LayerId) -> Option<Layer> {
        if id == self.root_layer.id {
            return None;
        }
        let removed = remove_from_subtree(&mut self.root_layer, id);
        if removed.is_some() {
            self.dirty = true;
            if self.active_layer_id == Some(id) {
                self.active_layer_id = None;
            }
        }
        removed
    }

    /// Find a layer by id (read-only).
    pub fn find_layer(&self, id: LayerId) -> Option<&Layer> {
        self.root_layer.find(id)
    }

    /// Find a layer by id (mutable).
    pub fn find_layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.root_layer.find_mut(id)
    }

    /// Set the active (selected) layer.
    pub fn set_active_layer(&mut self, id: LayerId) {
        self.active_layer_id = Some(id);
    }

    /// Total number of layers (including the root).
    pub fn layer_count(&self) -> usize {
        self.root_layer.subtree_count()
    }

    /// Depth-first iterator over all layers (including root).
    pub fn all_layers(&self) -> impl Iterator<Item = &Layer> {
        self.root_layer.depth_first_iter()
    }

    /// Mark the document as having unsaved changes.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Whether the document has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

/// Recursively search the subtree rooted at `layer` for a child with the
/// given id, remove it, and return it.
fn remove_from_subtree(layer: &mut Layer, target_id: LayerId) -> Option<Layer> {
    // Check direct children first.
    if let Some(pos) = layer.children.iter().position(|c| c.id == target_id) {
        let mut removed = layer.children.remove(pos);
        removed.parent_id = None;
        return Some(removed);
    }
    // Recurse into children.
    for child in &mut layer.children {
        if let Some(removed) = remove_from_subtree(child, target_id) {
            return Some(removed);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_has_root() {
        let doc = Document::new("Untitled", 1920, 1080);
        assert_eq!(doc.name, "Untitled");
        assert_eq!(doc.width, 1920);
        assert_eq!(doc.height, 1080);
        assert!(!doc.is_dirty());
        assert_eq!(doc.layer_count(), 1); // root only
        assert!(doc.root_layer.is_folder());
    }

    #[test]
    fn add_layer_to_root() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.add_layer("Paint1", LayerKind::Paint, None);
        assert_eq!(doc.layer_count(), 2);
        assert!(doc.is_dirty());

        let layer = doc.find_layer(id).unwrap();
        assert_eq!(layer.name, "Paint1");
        assert_eq!(layer.parent_id, Some(doc.root_layer.id));
    }

    #[test]
    fn add_layer_to_subfolder() {
        let mut doc = Document::new("Test", 100, 100);
        let folder_id = doc.add_layer("Group", LayerKind::Folder, None);
        let child_id = doc.add_layer("InGroup", LayerKind::Paint, Some(folder_id));

        assert_eq!(doc.layer_count(), 3);
        let child = doc.find_layer(child_id).unwrap();
        assert_eq!(child.parent_id, Some(folder_id));
    }

    #[test]
    fn remove_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.add_layer("ToRemove", LayerKind::Paint, None);
        doc.set_active_layer(id);

        let removed = doc.remove_layer(id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "ToRemove");
        assert_eq!(doc.layer_count(), 1);
        assert!(doc.active_layer_id.is_none());
    }

    #[test]
    fn remove_nested_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let folder_id = doc.add_layer("Group", LayerKind::Folder, None);
        let child_id = doc.add_layer("Deep", LayerKind::Paint, Some(folder_id));

        let removed = doc.remove_layer(child_id);
        assert!(removed.is_some());
        assert_eq!(doc.layer_count(), 2); // root + folder
    }

    #[test]
    fn cannot_remove_root() {
        let mut doc = Document::new("Test", 100, 100);
        let root_id = doc.root_layer.id;
        assert!(doc.remove_layer(root_id).is_none());
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut doc = Document::new("Test", 100, 100);
        let bogus = LayerId::new();
        assert!(doc.remove_layer(bogus).is_none());
    }

    #[test]
    fn all_layers_iterates_tree() {
        let mut doc = Document::new("Test", 100, 100);
        doc.add_layer("A", LayerKind::Paint, None);
        doc.add_layer("B", LayerKind::Paint, None);

        let names: Vec<&str> = doc.all_layers().map(|l| l.name.as_str()).collect();
        assert_eq!(names, vec!["Root", "A", "B"]);
    }

    #[test]
    fn find_layer_mut_modifies() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.add_layer("Layer1", LayerKind::Paint, None);

        doc.find_layer_mut(id).unwrap().opacity = 0.5;
        assert!((doc.find_layer(id).unwrap().opacity - 0.5).abs() < 1e-6);
    }

    #[test]
    fn serialization_roundtrip() {
        let mut doc = Document::new("MyDoc", 800, 600);
        let layer_id = doc.add_layer("Paint", LayerKind::Paint, None);
        doc.set_active_layer(layer_id);
        doc.mark_dirty();

        let json = serde_json::to_string_pretty(&doc).unwrap();
        let restored: Document = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.name, "MyDoc");
        assert_eq!(restored.width, 800);
        assert_eq!(restored.height, 600);
        assert_eq!(restored.active_layer_id, Some(layer_id));
        assert_eq!(restored.layer_count(), 2);
        // `dirty` is skipped during serialization
        assert!(!restored.is_dirty());
    }

    #[test]
    fn set_active_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.add_layer("Target", LayerKind::Paint, None);
        doc.set_active_layer(id);
        assert_eq!(doc.active_layer_id, Some(id));
    }
}
