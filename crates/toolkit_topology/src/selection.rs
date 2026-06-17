use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::halfedge::{EdgeId, FaceId, VertexId};

/// Which kind of element a topology selection targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectMode {
    Vertex,
    Edge,
    Face,
}

/// Selection sets over a [`HalfEdgeMesh`](crate::HalfEdgeMesh): independent
/// vertex, edge, and face sets plus the active mode. This drives editing tools
/// and, importantly, UV seam marking (a seam is simply a set of selected edges).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshSelection {
    pub mode: SelectMode,
    vertices: HashSet<VertexId>,
    edges: HashSet<EdgeId>,
    faces: HashSet<FaceId>,
}

impl Default for MeshSelection {
    fn default() -> Self {
        Self {
            mode: SelectMode::Vertex,
            vertices: HashSet::new(),
            edges: HashSet::new(),
            faces: HashSet::new(),
        }
    }
}

impl MeshSelection {
    pub fn new(mode: SelectMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.edges.clear();
        self.faces.clear();
    }

    // -- Vertices ------------------------------------------------------------
    pub fn add_vertex(&mut self, v: VertexId) {
        self.vertices.insert(v);
    }
    pub fn toggle_vertex(&mut self, v: VertexId) -> bool {
        if !self.vertices.remove(&v) {
            self.vertices.insert(v);
            true
        } else {
            false
        }
    }
    pub fn has_vertex(&self, v: VertexId) -> bool {
        self.vertices.contains(&v)
    }
    pub fn vertices(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.vertices.iter().copied()
    }

    // -- Edges ---------------------------------------------------------------
    pub fn add_edge(&mut self, e: EdgeId) {
        self.edges.insert(e);
    }
    pub fn toggle_edge(&mut self, e: EdgeId) -> bool {
        if !self.edges.remove(&e) {
            self.edges.insert(e);
            true
        } else {
            false
        }
    }
    pub fn has_edge(&self, e: EdgeId) -> bool {
        self.edges.contains(&e)
    }
    pub fn edges(&self) -> impl Iterator<Item = EdgeId> + '_ {
        self.edges.iter().copied()
    }

    // -- Faces ---------------------------------------------------------------
    pub fn add_face(&mut self, f: FaceId) {
        self.faces.insert(f);
    }
    pub fn toggle_face(&mut self, f: FaceId) -> bool {
        if !self.faces.remove(&f) {
            self.faces.insert(f);
            true
        } else {
            false
        }
    }
    pub fn has_face(&self, f: FaceId) -> bool {
        self.faces.contains(&f)
    }
    pub fn faces(&self) -> impl Iterator<Item = FaceId> + '_ {
        self.faces.iter().copied()
    }

    pub fn count(&self) -> usize {
        match self.mode {
            SelectMode::Vertex => self.vertices.len(),
            SelectMode::Edge => self.edges.len(),
            SelectMode::Face => self.faces.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_round_trip() {
        let mut sel = MeshSelection::new(SelectMode::Edge);
        assert!(sel.toggle_edge(5));
        assert!(sel.has_edge(5));
        assert!(!sel.toggle_edge(5));
        assert!(!sel.has_edge(5));
    }

    #[test]
    fn independent_sets() {
        let mut sel = MeshSelection::default();
        sel.add_vertex(1);
        sel.add_edge(2);
        sel.add_face(3);
        assert!(sel.has_vertex(1));
        assert!(sel.has_edge(2));
        assert!(sel.has_face(3));
    }

    #[test]
    fn count_follows_mode() {
        let mut sel = MeshSelection::new(SelectMode::Face);
        sel.add_face(0);
        sel.add_face(1);
        sel.add_vertex(9);
        assert_eq!(sel.count(), 2);
        sel.mode = SelectMode::Vertex;
        assert_eq!(sel.count(), 1);
    }
}
