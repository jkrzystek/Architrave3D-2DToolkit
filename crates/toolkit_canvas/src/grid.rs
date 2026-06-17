use glam::Vec2;

use crate::view::CanvasView;

/// Choose a "nice" grid spacing (1, 2, 5 × 10ⁿ) in canvas units such that one
/// cell is at least `target_px` pixels on screen. This keeps the grid readable
/// at any zoom level.
pub fn adaptive_step(view: &CanvasView, target_px: f32) -> f32 {
    let target_canvas = (target_px / view.zoom).max(1e-9);
    let exponent = target_canvas.log10().floor();
    let base = 10f32.powf(exponent);
    let normalized = target_canvas / base; // in [1, 10)
    let nice = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

/// Canvas-space coordinates of the vertical (x=) and horizontal (y=) grid lines
/// visible in the viewport, at the given spacing.
pub fn grid_lines(view: &CanvasView, step: f32) -> (Vec<f32>, Vec<f32>) {
    let (min, max) = view.visible_bounds();
    let step = step.max(1e-9);

    let mut xs = Vec::new();
    let mut x = (min.x / step).floor() * step;
    while x <= max.x {
        xs.push(x);
        x += step;
        if xs.len() > 10_000 {
            break; // guard against degenerate zoom
        }
    }

    let mut ys = Vec::new();
    let mut y = (min.y / step).floor() * step;
    while y <= max.y {
        ys.push(y);
        y += step;
        if ys.len() > 10_000 {
            break;
        }
    }
    (xs, ys)
}

/// Snap a canvas-space point to the nearest grid intersection.
pub fn snap(point: Vec2, step: f32) -> Vec2 {
    if step <= 1e-9 {
        return point;
    }
    Vec2::new(
        (point.x / step).round() * step,
        (point.y / step).round() * step,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_step_is_nice_number() {
        let view = CanvasView {
            zoom: 1.0,
            ..Default::default()
        };
        let step = adaptive_step(&view, 50.0);
        // 50 canvas units target -> nice value 50.
        assert!((step - 50.0).abs() < 1e-3, "step = {step}");
    }

    #[test]
    fn adaptive_step_scales_with_zoom() {
        let mut view = CanvasView::default();
        view.zoom = 100.0; // very zoomed in
        let step = adaptive_step(&view, 50.0);
        // Target 0.5 canvas units -> nice value 0.5.
        assert!((step - 0.5).abs() < 1e-4, "step = {step}");
    }

    #[test]
    fn grid_lines_cover_visible_region() {
        let view = CanvasView {
            center: Vec2::ZERO,
            zoom: 1.0,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        };
        let (xs, ys) = grid_lines(&view, 100.0);
        // Visible x spans roughly [-400, 400] -> lines at -400..400 step 100.
        assert!(xs.iter().any(|&x| (x - 0.0).abs() < 1e-3));
        assert!(xs.len() >= 8 && ys.len() >= 6);
    }

    #[test]
    fn snap_rounds_to_grid() {
        assert_eq!(snap(Vec2::new(12.0, 27.0), 10.0), Vec2::new(10.0, 30.0));
    }
}
