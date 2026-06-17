use egui::{Color32, Style, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolkitTheme {
    pub mode: ThemeMode,
    pub accent_color: [u8; 3],
    pub viewport_background: [u8; 3],
    pub panel_rounding: f32,
    pub font_size_body: f32,
    pub font_size_heading: f32,
}

impl Default for ToolkitTheme {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
            accent_color: [80, 140, 220],
            viewport_background: [40, 40, 45],
            panel_rounding: 4.0,
            font_size_body: 13.0,
            font_size_heading: 16.0,
        }
    }
}

impl ToolkitTheme {
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            accent_color: [50, 110, 200],
            viewport_background: [180, 180, 185],
            panel_rounding: 4.0,
            font_size_body: 13.0,
            font_size_heading: 16.0,
        }
    }

    pub fn accent(&self) -> Color32 {
        Color32::from_rgb(self.accent_color[0], self.accent_color[1], self.accent_color[2])
    }

    pub fn viewport_bg(&self) -> Color32 {
        Color32::from_rgb(
            self.viewport_background[0],
            self.viewport_background[1],
            self.viewport_background[2],
        )
    }

    pub fn apply_to_egui(&self) -> Style {
        let mut style = match self.mode {
            ThemeMode::Dark => Style {
                visuals: Visuals::dark(),
                ..Default::default()
            },
            ThemeMode::Light => Style {
                visuals: Visuals::light(),
                ..Default::default()
            },
        };

        style.visuals.selection.bg_fill = self.accent();

        style
    }
}

pub fn separator_line(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    ui.separator();
    ui.add_space(2.0);
}

pub fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.strong(text);
    ui.add_space(2.0);
}

pub fn labeled_value(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.monospace(value);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_dark() {
        let theme = ToolkitTheme::default();
        assert_eq!(theme.mode, ThemeMode::Dark);
    }

    #[test]
    fn light_theme() {
        let theme = ToolkitTheme::light();
        assert_eq!(theme.mode, ThemeMode::Light);
    }

    #[test]
    fn accent_color() {
        let theme = ToolkitTheme::default();
        let c = theme.accent();
        assert_eq!(c, Color32::from_rgb(80, 140, 220));
    }

    #[test]
    fn theme_serialization_roundtrip() {
        let theme = ToolkitTheme::default();
        let json = serde_json::to_string(&theme).unwrap();
        let back: ToolkitTheme = serde_json::from_str(&json).unwrap();
        assert_eq!(back.mode, theme.mode);
        assert_eq!(back.accent_color, theme.accent_color);
    }

    #[test]
    fn apply_to_egui_produces_style() {
        let theme = ToolkitTheme::default();
        let style = theme.apply_to_egui();
        assert_eq!(style.visuals.selection.bg_fill, theme.accent());
    }
}
