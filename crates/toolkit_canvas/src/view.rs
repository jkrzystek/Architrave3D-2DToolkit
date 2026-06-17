use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A 2D pan/zoom view — the "camera" of a 2D editor (paint canvas, UV editor,
/// node graph, timeline). Maps between **canvas space** (the document's own
/// coordinates) and **screen space** (pixels in the viewport).
///
/// The mapping is:
/// `screen = (canvas - center) * zoom + viewport/2`
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CanvasView {
    /// Canvas-space point shown at the centre of the viewport.
    pub center: Vec2,
    /// Pixels per canvas unit. Larger = more zoomed in.
    pub zoom: f32,
    /// Viewport size in pixels.
    pub viewport: Vec2,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            zoom: 1.0,
            viewport: Vec2::new(800.0, 600.0),
            min_zoom: 0.01,
            max_zoom: 256.0,
        }
    }
}

impl CanvasView {
    pub fn new(viewport: Vec2) -> Self {
        Self {
            viewport,
            ..Default::default()
        }
    }

    pub fn canvas_to_screen(&self, canvas: Vec2) -> Vec2 {
        (canvas - self.center) * self.zoom + self.viewport * 0.5
    }

    pub fn screen_to_canvas(&self, screen: Vec2) -> Vec2 {
        (screen - self.viewport * 0.5) / self.zoom + self.center
    }

    /// Pan by a screen-space pixel delta (e.g. a drag with the middle mouse).
    pub fn pan_pixels(&mut self, delta_px: Vec2) {
        self.center -= delta_px / self.zoom;
    }

    /// Zoom by `factor` while keeping the canvas point under `screen_anchor`
    /// fixed on screen — the standard scroll-wheel-to-cursor behaviour.
    pub fn zoom_at(&mut self, screen_anchor: Vec2, factor: f32) {
        let anchor_canvas = self.screen_to_canvas(screen_anchor);
        self.zoom = (self.zoom * factor).clamp(self.min_zoom, self.max_zoom);
        // Re-solve center so anchor_canvas still maps to screen_anchor.
        self.center = anchor_canvas - (screen_anchor - self.viewport * 0.5) / self.zoom;
    }

    /// Frame a canvas-space rectangle so it fills the viewport with `padding`
    /// (a fraction, e.g. 0.1 for 10% margin).
    pub fn fit_bounds(&mut self, min: Vec2, max: Vec2, padding: f32) {
        let size = (max - min).max(Vec2::splat(1e-6));
        self.center = (min + max) * 0.5;
        let pad = 1.0 + padding.max(0.0);
        let zoom_x = self.viewport.x / (size.x * pad);
        let zoom_y = self.viewport.y / (size.y * pad);
        self.zoom = zoom_x.min(zoom_y).clamp(self.min_zoom, self.max_zoom);
    }

    /// The canvas-space rectangle currently visible (min, max).
    pub fn visible_bounds(&self) -> (Vec2, Vec2) {
        let tl = self.screen_to_canvas(Vec2::ZERO);
        let br = self.screen_to_canvas(self.viewport);
        (tl.min(br), tl.max(br))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn round_trip_screen_canvas() {
        let view = CanvasView {
            center: Vec2::new(10.0, 20.0),
            zoom: 2.5,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        };
        let p = Vec2::new(123.0, 456.0);
        let back = view.canvas_to_screen(view.screen_to_canvas(p));
        assert!(approx(back, p));
    }

    #[test]
    fn center_maps_to_viewport_middle() {
        let view = CanvasView::new(Vec2::new(800.0, 600.0));
        assert!(approx(
            view.canvas_to_screen(view.center),
            Vec2::new(400.0, 300.0)
        ));
    }

    #[test]
    fn zoom_at_keeps_anchor_fixed() {
        let mut view = CanvasView::new(Vec2::new(800.0, 600.0));
        let anchor = Vec2::new(600.0, 200.0);
        let before = view.screen_to_canvas(anchor);
        view.zoom_at(anchor, 2.0);
        let after = view.screen_to_canvas(anchor);
        assert!(approx(before, after), "{before:?} != {after:?}");
        assert!((view.zoom - 2.0).abs() < 1e-4);
    }

    #[test]
    fn pan_moves_center() {
        let mut view = CanvasView::new(Vec2::new(800.0, 600.0));
        let start = view.center;
        view.pan_pixels(Vec2::new(100.0, 0.0)); // drag right
        assert!(view.center.x < start.x);
    }

    #[test]
    fn fit_bounds_frames_region() {
        let mut view = CanvasView::new(Vec2::new(800.0, 600.0));
        view.fit_bounds(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0), 0.0);
        assert!(approx(view.center, Vec2::new(50.0, 50.0)));
        // The 100x100 region must fit within the 800x600 viewport.
        let (min, max) = view.visible_bounds();
        assert!(min.x <= 0.0 + 1e-3 && max.x >= 100.0 - 1e-3);
    }

    #[test]
    fn zoom_clamps() {
        let mut view = CanvasView::new(Vec2::new(800.0, 600.0));
        view.zoom_at(Vec2::new(400.0, 300.0), 10000.0);
        assert!(view.zoom <= view.max_zoom);
    }
}
