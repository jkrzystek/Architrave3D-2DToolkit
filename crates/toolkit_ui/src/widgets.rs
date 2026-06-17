use egui::{Color32, Response, Sense, Ui, Vec2};

pub fn color_swatch(ui: &mut Ui, color: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 2.0, color);
        painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, Color32::GRAY), egui::StrokeKind::Outside);
    }
    response
}

pub fn slider_with_label(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> Response {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(value, range).show_value(true))
    })
    .inner
}

pub fn percentage_slider(ui: &mut Ui, label: &str, value: &mut f32) -> Response {
    let mut pct = *value * 100.0;
    let response = ui.horizontal(|ui| {
        ui.label(label);
        let r = ui.add(
            egui::Slider::new(&mut pct, 0.0..=100.0)
                .suffix("%")
                .show_value(true),
        );
        r
    });
    *value = pct / 100.0;
    response.inner
}

pub fn toggle_button(ui: &mut Ui, label: &str, active: &mut bool) -> Response {
    let text = if *active {
        egui::RichText::new(label).strong()
    } else {
        egui::RichText::new(label)
    };

    let response = ui.selectable_label(*active, text);
    if response.clicked() {
        *active = !*active;
    }
    response
}

pub struct PropertyGrid<'a> {
    ui: &'a mut Ui,
    label_width: f32,
}

impl<'a> PropertyGrid<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        Self {
            ui,
            label_width: 80.0,
        }
    }

    pub fn with_label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }

    pub fn row(&mut self, label: &str, add_contents: impl FnOnce(&mut Ui)) {
        self.ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(self.label_width, ui.spacing().interact_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(label);
                },
            );
            add_contents(ui);
        });
    }

    pub fn float_row(
        &mut self,
        label: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
    ) {
        self.row(label, |ui| {
            ui.add(egui::DragValue::new(value).range(range).speed(0.01));
        });
    }

    pub fn bool_row(&mut self, label: &str, value: &mut bool) {
        self.row(label, |ui| {
            ui.checkbox(value, "");
        });
    }

    pub fn text_row(&mut self, label: &str, value: &mut String) {
        self.row(label, |ui| {
            ui.text_edit_singleline(value);
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn percentage_conversion() {
        let mut val = 0.5f32;
        let pct = val * 100.0;
        assert!((pct - 50.0).abs() < 1e-5);
        val = pct / 100.0;
        assert!((val - 0.5).abs() < 1e-5);
    }
}
