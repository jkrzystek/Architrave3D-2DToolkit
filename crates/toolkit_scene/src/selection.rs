use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::node::NodeKey;

/// A set of selected scene nodes with a distinguished "active" node.
///
/// The active node is the most-recently-added member and is what tools like
/// gizmos and property panels operate on when a single target is needed, even
/// though the whole set may be transformed together.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Selection {
    members: HashSet<NodeKey>,
    active: Option<NodeKey>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the selection with a single node.
    pub fn select_only(&mut self, key: NodeKey) {
        self.members.clear();
        self.members.insert(key);
        self.active = Some(key);
    }

    /// Add a node to the selection and make it active.
    pub fn add(&mut self, key: NodeKey) {
        self.members.insert(key);
        self.active = Some(key);
    }

    /// Remove a node from the selection.
    pub fn remove(&mut self, key: NodeKey) {
        self.members.remove(&key);
        if self.active == Some(key) {
            self.active = self.members.iter().copied().next();
        }
    }

    /// Toggle a node's membership; returns the new membership state.
    pub fn toggle(&mut self, key: NodeKey) -> bool {
        if self.members.contains(&key) {
            self.remove(key);
            false
        } else {
            self.add(key);
            true
        }
    }

    pub fn clear(&mut self) {
        self.members.clear();
        self.active = None;
    }

    pub fn contains(&self, key: NodeKey) -> bool {
        self.members.contains(&key)
    }

    pub fn active(&self) -> Option<NodeKey> {
        self.active
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = NodeKey> + '_ {
        self.members.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NodeData, Scene, Transform};

    fn make_keys() -> (Scene, NodeKey, NodeKey) {
        let mut scene = Scene::new();
        let a = scene.add_node("a", Transform::IDENTITY, NodeData::Empty);
        let b = scene.add_node("b", Transform::IDENTITY, NodeData::Empty);
        (scene, a, b)
    }

    #[test]
    fn select_only_sets_active() {
        let (_s, a, _b) = make_keys();
        let mut sel = Selection::new();
        sel.add(a);
        sel.select_only(a);
        assert_eq!(sel.len(), 1);
        assert_eq!(sel.active(), Some(a));
    }

    #[test]
    fn toggle_adds_and_removes() {
        let (_s, a, _b) = make_keys();
        let mut sel = Selection::new();
        assert!(sel.toggle(a));
        assert!(sel.contains(a));
        assert!(!sel.toggle(a));
        assert!(!sel.contains(a));
    }

    #[test]
    fn removing_active_picks_new_active() {
        let (_s, a, b) = make_keys();
        let mut sel = Selection::new();
        sel.add(a);
        sel.add(b);
        assert_eq!(sel.active(), Some(b));
        sel.remove(b);
        assert_eq!(sel.active(), Some(a));
    }

    #[test]
    fn clear_empties() {
        let (_s, a, b) = make_keys();
        let mut sel = Selection::new();
        sel.add(a);
        sel.add(b);
        sel.clear();
        assert!(sel.is_empty());
        assert_eq!(sel.active(), None);
    }
}
