use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use toolkit_geometry::Ray;

use crate::math::{closest_param_on_line, ray_line_distance, ray_plane_intersection};

/// The transform operation the gizmo performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

/// Which handle of the gizmo is being addressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
    /// Planar handles (translate only): drag within a coordinate plane.
    XY,
    YZ,
    XZ,
    /// View-aligned handle: uniform scale or screen-space translate/rotate.
    Screen,
}

/// A pickable handle: a (mode, axis) pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GizmoHandle {
    pub mode: GizmoMode,
    pub axis: GizmoAxis,
}

/// The cumulative transform produced by a drag, relative to the transform at
/// drag start. The application applies it to the object's start transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GizmoDelta {
    /// World-space translation to add.
    Translate(Vec3),
    /// Rotation to compose (left-multiply) onto the start orientation.
    Rotate(Quat),
    /// Per-axis scale multiplier (1.0 = unchanged).
    Scale(Vec3),
}

/// Visual/interaction sizing in world units.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GizmoConfig {
    /// Length of the axis handles.
    pub size: f32,
    /// Pick radius around axis lines.
    pub handle_thickness: f32,
    /// Radius of the rotation rings (as a fraction of `size`).
    pub ring_radius: f32,
    /// Pick tolerance for rotation rings.
    pub ring_thickness: f32,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            size: 1.0,
            handle_thickness: 0.1,
            ring_radius: 1.0,
            ring_thickness: 0.1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DragState {
    handle: GizmoHandle,
    /// Reference scalar (param along axis or start angle) captured at begin.
    start_scalar: f32,
    /// Reference point (for planar translate).
    start_point: Vec3,
}

/// A transform gizmo. Holds its world placement and current mode; performs
/// hit-testing against a ray and converts drags into [`GizmoDelta`]s. It does
/// **not** render anything — the application draws handles however it likes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gizmo {
    pub origin: Vec3,
    /// Orientation of the gizmo's local axes (identity = world-aligned).
    pub orientation: Quat,
    pub mode: GizmoMode,
    pub config: GizmoConfig,
    #[serde(skip)]
    drag: Option<DragState>,
}

impl Gizmo {
    pub fn new(origin: Vec3, mode: GizmoMode) -> Self {
        Self {
            origin,
            orientation: Quat::IDENTITY,
            mode,
            config: GizmoConfig::default(),
            drag: None,
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// World-space direction of an axis handle.
    pub fn axis_dir(&self, axis: GizmoAxis) -> Vec3 {
        match axis {
            GizmoAxis::X => self.orientation * Vec3::X,
            GizmoAxis::Y => self.orientation * Vec3::Y,
            GizmoAxis::Z => self.orientation * Vec3::Z,
            _ => Vec3::ZERO,
        }
    }

    /// Plane normal for a planar/rotation handle.
    fn plane_normal(&self, axis: GizmoAxis, view_dir: Vec3) -> Vec3 {
        match axis {
            GizmoAxis::X | GizmoAxis::YZ => self.orientation * Vec3::X,
            GizmoAxis::Y | GizmoAxis::XZ => self.orientation * Vec3::Y,
            GizmoAxis::Z | GizmoAxis::XY => self.orientation * Vec3::Z,
            GizmoAxis::Screen => view_dir,
        }
    }

    // -- Hit testing ---------------------------------------------------------

    /// Pick the handle under `ray`. `view_dir` is the camera forward vector
    /// (used by the screen-space handle). Returns `None` if nothing is hit.
    pub fn hit_test(&self, ray: &Ray, view_dir: Vec3) -> Option<GizmoHandle> {
        match self.mode {
            GizmoMode::Translate => self.hit_translate(ray, view_dir),
            GizmoMode::Scale => self.hit_axes(ray, GizmoMode::Scale),
            GizmoMode::Rotate => self.hit_rotate(ray, view_dir),
        }
    }

    fn hit_axes(&self, ray: &Ray, mode: GizmoMode) -> Option<GizmoHandle> {
        let mut best: Option<(f32, GizmoAxis)> = None;
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let dir = self.axis_dir(axis);
            // Only consider the positive handle segment [0, size].
            if let Some(s) = closest_param_on_line(ray, self.origin, dir) {
                if s < -self.config.handle_thickness || s > self.config.size {
                    continue;
                }
                let dist = ray_line_distance(ray, self.origin, dir);
                if dist < self.config.handle_thickness
                    && best.map(|(d, _)| dist < d).unwrap_or(true)
                {
                    best = Some((dist, axis));
                }
            }
        }
        best.map(|(_, axis)| GizmoHandle { mode, axis })
    }

    fn hit_translate(&self, ray: &Ray, view_dir: Vec3) -> Option<GizmoHandle> {
        // Planar handles take priority near the origin, then the axes.
        for axis in [GizmoAxis::XY, GizmoAxis::YZ, GizmoAxis::XZ] {
            let n = self.plane_normal(axis, view_dir);
            if let Some(hit) = ray_plane_intersection(ray, self.origin, n) {
                let local = hit - self.origin;
                let (u, v) = self.plane_axes(axis);
                let a = local.dot(u);
                let b = local.dot(v);
                let lo = self.config.size * 0.2;
                let hi = self.config.size * 0.5;
                if a > lo && a < hi && b > lo && b < hi {
                    return Some(GizmoHandle {
                        mode: GizmoMode::Translate,
                        axis,
                    });
                }
            }
        }
        self.hit_axes(ray, GizmoMode::Translate)
    }

    fn plane_axes(&self, axis: GizmoAxis) -> (Vec3, Vec3) {
        match axis {
            GizmoAxis::XY => (self.axis_dir(GizmoAxis::X), self.axis_dir(GizmoAxis::Y)),
            GizmoAxis::YZ => (self.axis_dir(GizmoAxis::Y), self.axis_dir(GizmoAxis::Z)),
            GizmoAxis::XZ => (self.axis_dir(GizmoAxis::X), self.axis_dir(GizmoAxis::Z)),
            _ => (Vec3::X, Vec3::Y),
        }
    }

    fn hit_rotate(&self, ray: &Ray, view_dir: Vec3) -> Option<GizmoHandle> {
        let radius = self.config.size * self.config.ring_radius;
        let mut best: Option<(f32, GizmoAxis)> = None;
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let n = self.axis_dir(axis);
            if let Some(hit) = ray_plane_intersection(ray, self.origin, n) {
                let r = (hit - self.origin).length();
                let err = (r - radius).abs();
                if err < self.config.ring_thickness * self.config.size
                    && best.map(|(e, _)| err < e).unwrap_or(true)
                {
                    best = Some((err, axis));
                }
            }
        }
        best.map(|(_, axis)| GizmoHandle {
            mode: GizmoMode::Rotate,
            axis,
        })
        .or_else(|| {
            // Screen-space ring as a fallback.
            ray_plane_intersection(ray, self.origin, view_dir).and_then(|hit| {
                let r = (hit - self.origin).length();
                if (r - radius * 1.2).abs() < self.config.ring_thickness * self.config.size {
                    Some(GizmoHandle {
                        mode: GizmoMode::Rotate,
                        axis: GizmoAxis::Screen,
                    })
                } else {
                    None
                }
            })
        })
    }

    // -- Dragging ------------------------------------------------------------

    /// Begin a drag on `handle`, capturing reference state from the ray.
    pub fn begin_drag(&mut self, handle: GizmoHandle, ray: &Ray, view_dir: Vec3) {
        let (scalar, point) = self.reference(handle, ray, view_dir);
        self.drag = Some(DragState {
            handle,
            start_scalar: scalar,
            start_point: point,
        });
    }

    /// Update the active drag with a new ray. Returns the cumulative delta from
    /// drag start, or `None` if no drag is active.
    pub fn update_drag(&mut self, ray: &Ray, view_dir: Vec3) -> Option<GizmoDelta> {
        let drag = self.drag?;
        let (scalar, point) = self.reference(drag.handle, ray, view_dir);
        let delta = match drag.handle.mode {
            GizmoMode::Translate => match drag.handle.axis {
                GizmoAxis::X | GizmoAxis::Y | GizmoAxis::Z => {
                    let dir = self.axis_dir(drag.handle.axis);
                    GizmoDelta::Translate(dir * (scalar - drag.start_scalar))
                }
                _ => GizmoDelta::Translate(point - drag.start_point),
            },
            GizmoMode::Rotate => {
                let axis = if drag.handle.axis == GizmoAxis::Screen {
                    view_dir
                } else {
                    self.axis_dir(drag.handle.axis)
                };
                GizmoDelta::Rotate(Quat::from_axis_angle(
                    axis.normalize(),
                    scalar - drag.start_scalar,
                ))
            }
            GizmoMode::Scale => {
                let factor = 1.0 + (scalar - drag.start_scalar) / self.config.size;
                let v = match drag.handle.axis {
                    GizmoAxis::X => Vec3::new(factor, 1.0, 1.0),
                    GizmoAxis::Y => Vec3::new(1.0, factor, 1.0),
                    GizmoAxis::Z => Vec3::new(1.0, 1.0, factor),
                    _ => Vec3::splat(factor),
                };
                GizmoDelta::Scale(v)
            }
        };
        Some(delta)
    }

    pub fn end_drag(&mut self) {
        self.drag = None;
    }

    /// Compute the reference scalar/point for a handle from a ray.
    fn reference(&self, handle: GizmoHandle, ray: &Ray, view_dir: Vec3) -> (f32, Vec3) {
        match handle.mode {
            GizmoMode::Translate | GizmoMode::Scale => match handle.axis {
                GizmoAxis::X | GizmoAxis::Y | GizmoAxis::Z => {
                    let dir = self.axis_dir(handle.axis);
                    let s = closest_param_on_line(ray, self.origin, dir).unwrap_or(0.0);
                    (s, self.origin + dir * s)
                }
                _ => {
                    let n = self.plane_normal(handle.axis, view_dir);
                    let p = ray_plane_intersection(ray, self.origin, n).unwrap_or(self.origin);
                    (0.0, p)
                }
            },
            GizmoMode::Rotate => {
                let axis = if handle.axis == GizmoAxis::Screen {
                    view_dir
                } else {
                    self.axis_dir(handle.axis)
                };
                let hit = ray_plane_intersection(ray, self.origin, axis).unwrap_or(self.origin);
                let local = hit - self.origin;
                // Build an in-plane basis to measure the angle.
                let (u, v) = plane_basis(axis);
                let angle = local.dot(v).atan2(local.dot(u));
                (angle, hit)
            }
        }
    }
}

fn plane_basis(normal: Vec3) -> (Vec3, Vec3) {
    let n = normal.normalize_or_zero();
    let helper = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = helper.cross(n).normalize_or_zero();
    let v = n.cross(u).normalize_or_zero();
    (u, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down_ray(x: f32, z: f32) -> Ray {
        Ray::new(Vec3::new(x, 5.0, z), Vec3::new(0.0, -1.0, 0.0))
    }

    #[test]
    fn translate_hit_x_axis() {
        let g = Gizmo::new(Vec3::ZERO, GizmoMode::Translate);
        // Ray straight down onto a point on the +X handle.
        let hit = g.hit_test(&down_ray(0.5, 0.0), Vec3::NEG_Y).unwrap();
        assert_eq!(hit.axis, GizmoAxis::X);
        assert_eq!(hit.mode, GizmoMode::Translate);
    }

    #[test]
    fn translate_miss() {
        let g = Gizmo::new(Vec3::ZERO, GizmoMode::Translate);
        // Far from any handle.
        assert!(g.hit_test(&down_ray(5.0, 5.0), Vec3::NEG_Y).is_none());
    }

    #[test]
    fn translate_drag_along_x() {
        let mut g = Gizmo::new(Vec3::ZERO, GizmoMode::Translate);
        let handle = GizmoHandle {
            mode: GizmoMode::Translate,
            axis: GizmoAxis::X,
        };
        g.begin_drag(handle, &down_ray(0.5, 0.0), Vec3::NEG_Y);
        let delta = g.update_drag(&down_ray(0.8, 0.0), Vec3::NEG_Y).unwrap();
        match delta {
            GizmoDelta::Translate(v) => {
                assert!((v - Vec3::new(0.3, 0.0, 0.0)).length() < 1e-4, "v = {v:?}");
            }
            _ => panic!("expected translate"),
        }
    }

    #[test]
    fn rotate_drag_about_z_quarter_turn() {
        let mut g = Gizmo::new(Vec3::ZERO, GizmoMode::Rotate);
        let handle = GizmoHandle {
            mode: GizmoMode::Rotate,
            axis: GizmoAxis::Z,
        };
        // Rays come down -Z onto the XY plane (normal = Z).
        let r0 = Ray::new(Vec3::new(1.0, 0.0, 5.0), Vec3::NEG_Z);
        let r1 = Ray::new(Vec3::new(0.0, 1.0, 5.0), Vec3::NEG_Z);
        g.begin_drag(handle, &r0, Vec3::NEG_Z);
        let delta = g.update_drag(&r1, Vec3::NEG_Z).unwrap();
        match delta {
            GizmoDelta::Rotate(q) => {
                // Rotating +X by this quat should land near +Y.
                let rotated = q * Vec3::X;
                assert!((rotated - Vec3::Y).length() < 1e-3, "rotated = {rotated:?}");
            }
            _ => panic!("expected rotate"),
        }
    }

    #[test]
    fn scale_drag_grows_axis() {
        let mut g = Gizmo::new(Vec3::ZERO, GizmoMode::Scale);
        let handle = GizmoHandle {
            mode: GizmoMode::Scale,
            axis: GizmoAxis::X,
        };
        g.begin_drag(handle, &down_ray(0.5, 0.0), Vec3::NEG_Y);
        let delta = g.update_drag(&down_ray(1.5, 0.0), Vec3::NEG_Y).unwrap();
        match delta {
            GizmoDelta::Scale(v) => {
                // Moved +1.0 along X over size 1.0 -> factor 2.0 on X only.
                assert!((v.x - 2.0).abs() < 1e-4, "v = {v:?}");
                assert!((v.y - 1.0).abs() < 1e-4);
            }
            _ => panic!("expected scale"),
        }
    }

    #[test]
    fn end_drag_clears_state() {
        let mut g = Gizmo::new(Vec3::ZERO, GizmoMode::Translate);
        let handle = GizmoHandle {
            mode: GizmoMode::Translate,
            axis: GizmoAxis::X,
        };
        g.begin_drag(handle, &down_ray(0.5, 0.0), Vec3::NEG_Y);
        assert!(g.is_dragging());
        g.end_drag();
        assert!(!g.is_dragging());
        assert!(g.update_drag(&down_ray(0.8, 0.0), Vec3::NEG_Y).is_none());
    }
}
