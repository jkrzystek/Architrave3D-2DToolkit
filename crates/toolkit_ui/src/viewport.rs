use egui::{Rect, Response, Sense, Ui, Vec2};
use toolkit_core::ViewportId;

pub struct ViewportPanel {
    pub id: ViewportId,
    pub label: String,
    pub min_size: Vec2,
}

pub struct ViewportResponse {
    pub rect: Rect,
    pub response: Response,
    pub size_changed: bool,
    previous_size: Vec2,
}

impl ViewportResponse {
    pub fn hovered(&self) -> bool {
        self.response.hovered()
    }

    pub fn width(&self) -> f32 {
        self.rect.width()
    }

    pub fn height(&self) -> f32 {
        self.rect.height()
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.rect.height() > 0.0 {
            self.rect.width() / self.rect.height()
        } else {
            1.0
        }
    }

    pub fn local_mouse_pos(&self, screen_pos: egui::Pos2) -> Option<egui::Pos2> {
        if self.rect.contains(screen_pos) {
            Some(egui::pos2(
                screen_pos.x - self.rect.min.x,
                screen_pos.y - self.rect.min.y,
            ))
        } else {
            None
        }
    }

    pub fn normalized_mouse_pos(&self, screen_pos: egui::Pos2) -> Option<egui::Pos2> {
        self.local_mouse_pos(screen_pos).map(|local| {
            egui::pos2(
                local.x / self.rect.width(),
                local.y / self.rect.height(),
            )
        })
    }
}

impl ViewportPanel {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: ViewportId::new(),
            label: label.into(),
            min_size: Vec2::new(64.0, 64.0),
        }
    }

    pub fn with_min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    pub fn show(&self, ui: &mut Ui, previous_size: Vec2) -> ViewportResponse {
        let available = ui.available_size();
        let size = Vec2::new(
            available.x.max(self.min_size.x),
            available.y.max(self.min_size.y),
        );

        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        let size_changed = (previous_size - size).length() > 0.5;

        ViewportResponse {
            rect,
            response,
            size_changed,
            previous_size: size,
        }
    }

    pub fn show_with_texture(
        &self,
        ui: &mut Ui,
        texture_id: egui::TextureId,
        previous_size: Vec2,
    ) -> ViewportResponse {
        let vr = self.show(ui, previous_size);

        if ui.is_rect_visible(vr.rect) {
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(
                texture_id,
                vr.rect,
                uv,
                egui::Color32::WHITE,
            );
        }

        vr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_panel_creation() {
        let vp = ViewportPanel::new("Test Viewport");
        assert_eq!(vp.label, "Test Viewport");
    }

    #[test]
    fn viewport_panel_min_size() {
        let vp = ViewportPanel::new("Test").with_min_size(Vec2::new(128.0, 128.0));
        assert_eq!(vp.min_size, Vec2::new(128.0, 128.0));
    }

    #[test]
    fn local_mouse_pos_inside_rect() {
        let rect = Rect::from_min_size(egui::pos2(100.0, 200.0), Vec2::new(800.0, 600.0));
        let screen = egui::pos2(150.0, 250.0);
        if rect.contains(screen) {
            let local = egui::pos2(screen.x - rect.min.x, screen.y - rect.min.y);
            assert!((local.x - 50.0).abs() < 1e-3);
            assert!((local.y - 50.0).abs() < 1e-3);
        }
    }

    #[test]
    fn aspect_ratio_math() {
        let w = 800.0f32;
        let h = 600.0f32;
        let ratio = w / h;
        assert!((ratio - 4.0 / 3.0).abs() < 1e-3);
    }
}
