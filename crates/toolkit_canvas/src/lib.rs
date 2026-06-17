//! 2D editor foundation.
//!
//! A 2D editor (paint canvas, **UV editor**, node graph, sprite sheet) needs the
//! same handful of primitives that this crate provides, independent of any UI
//! framework:
//!
//! * [`CanvasView`] — pan/zoom mapping between document and screen space.
//! * [`adaptive_grid`](grid) — a grid whose spacing stays readable at any zoom,
//!   plus snapping.
//! * [`SelectionDrag`] / [`Rect2`] — rubber-band selection.
//!
//! These compose with [`toolkit_uv`] to build a UV editor: unwrap charts, then
//! display/edit their UVs through a [`CanvasView`].
//!
//! ```
//! use glam::Vec2;
//! use toolkit_canvas::CanvasView;
//!
//! let mut view = CanvasView::new(Vec2::new(800.0, 600.0));
//! view.fit_bounds(Vec2::ZERO, Vec2::ONE, 0.1); // frame the [0,1] UV square
//! let screen = view.canvas_to_screen(Vec2::new(0.5, 0.5));
//! assert!((screen - Vec2::new(400.0, 300.0)).length() < 1.0);
//! ```

pub mod grid;
pub mod select;
pub mod view;

pub use grid::{adaptive_step, grid_lines, snap};
pub use select::{Rect2, SelectionDrag};
pub use view::CanvasView;
