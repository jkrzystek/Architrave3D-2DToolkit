use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PanelId(pub String);

impl PanelId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl std::fmt::Display for PanelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelState {
    pub id: PanelId,
    pub title: String,
    pub visible: bool,
    pub closable: bool,
}

impl PanelState {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let id_str: String = id.into();
        Self {
            id: PanelId::new(id_str),
            title: title.into(),
            visible: true,
            closable: true,
        }
    }

    pub fn with_closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }
}

pub trait PanelContent {
    fn title(&self) -> &str;
    fn ui(&mut self, ui: &mut egui::Ui);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    pub panels: Vec<PanelState>,
    pub name: String,
}

impl WorkspaceLayout {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            panels: Vec::new(),
            name: name.into(),
        }
    }

    pub fn add_panel(&mut self, panel: PanelState) {
        self.panels.push(panel);
    }

    pub fn find_panel(&self, id: &PanelId) -> Option<&PanelState> {
        self.panels.iter().find(|p| &p.id == id)
    }

    pub fn find_panel_mut(&mut self, id: &PanelId) -> Option<&mut PanelState> {
        self.panels.iter_mut().find(|p| &p.id == id)
    }

    pub fn set_visibility(&mut self, id: &PanelId, visible: bool) {
        if let Some(panel) = self.find_panel_mut(id) {
            panel.visible = visible;
        }
    }

    pub fn visible_panels(&self) -> impl Iterator<Item = &PanelState> {
        self.panels.iter().filter(|p| p.visible)
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        let mut layout = Self::new("Default");
        layout.add_panel(PanelState::new("viewport_3d", "3D Viewport").with_closable(false));
        layout.add_panel(PanelState::new("viewport_2d", "2D Canvas"));
        layout.add_panel(PanelState::new("layers", "Layers"));
        layout.add_panel(PanelState::new("properties", "Properties"));
        layout.add_panel(PanelState::new("tools", "Tools"));
        layout.add_panel(PanelState::new("color_picker", "Color Picker"));
        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_has_panels() {
        let layout = WorkspaceLayout::default();
        assert!(layout.panel_count() >= 4);
    }

    #[test]
    fn add_and_find_panel() {
        let mut layout = WorkspaceLayout::new("Test");
        layout.add_panel(PanelState::new("my_panel", "My Panel"));
        assert!(layout.find_panel(&PanelId::new("my_panel")).is_some());
    }

    #[test]
    fn set_visibility() {
        let mut layout = WorkspaceLayout::default();
        let id = PanelId::new("layers");
        layout.set_visibility(&id, false);
        assert!(!layout.find_panel(&id).unwrap().visible);
    }

    #[test]
    fn visible_panels_filter() {
        let mut layout = WorkspaceLayout::default();
        let total = layout.panel_count();
        layout.set_visibility(&PanelId::new("layers"), false);
        let visible: Vec<_> = layout.visible_panels().collect();
        assert_eq!(visible.len(), total - 1);
    }

    #[test]
    fn panel_serialization_roundtrip() {
        let layout = WorkspaceLayout::default();
        let json = serde_json::to_string(&layout).unwrap();
        let deserialized: WorkspaceLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.panel_count(), layout.panel_count());
        assert_eq!(deserialized.name, layout.name);
    }
}
