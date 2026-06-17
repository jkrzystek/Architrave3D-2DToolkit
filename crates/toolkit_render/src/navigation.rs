//! Camera navigation controllers beyond the orbit camera: a first-person fly
//! controller and helpers to frame (focus) the camera on an object.
//!
//! These operate purely on [`Camera`] math and take no input dependency, so the
//! application maps whatever keys/mouse it likes onto the methods here.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::camera::{Camera, OrbitController, Projection};

/// A first-person / free-fly camera controller (WASD + mouse look).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlyController {
    pub position: Vec3,
    /// Yaw in radians (around +Y).
    pub yaw: f32,
    /// Pitch in radians, clamped to avoid gimbal flip at the poles.
    pub pitch: f32,
    /// Movement units per second.
    pub move_speed: f32,
    /// Radians of rotation per unit of mouse delta.
    pub look_sensitivity: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,
}

impl Default for FlyController {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.0,
            move_speed: 5.0,
            look_sensitivity: 0.005,
            min_pitch: -std::f32::consts::FRAC_PI_2 + 0.01,
            max_pitch: std::f32::consts::FRAC_PI_2 - 0.01,
        }
    }
}

impl FlyController {
    /// Forward direction implied by the current yaw/pitch.
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or_zero()
    }

    /// Apply mouse-look deltas (in pixels); positive `dx` looks right, positive
    /// `dy` looks down.
    pub fn look(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * self.look_sensitivity;
        self.pitch = (self.pitch - dy * self.look_sensitivity).clamp(self.min_pitch, self.max_pitch);
    }

    /// Move along the local axes. `axes` components are typically in `[-1, 1]`:
    /// x = strafe (right), y = up (world), z = forward.
    pub fn move_local(&mut self, axes: Vec3, dt: f32) {
        let step = self.move_speed * dt;
        self.position += self.right() * axes.x * step;
        self.position += Vec3::Y * axes.y * step;
        self.position += self.forward() * axes.z * step;
    }

    /// Write the controller's pose into a camera.
    pub fn apply_to(&self, camera: &mut Camera) {
        camera.position = self.position;
        camera.target = self.position + self.forward();
        camera.up = Vec3::Y;
    }

    /// Initialise a fly controller from an existing camera pose.
    pub fn from_camera(camera: &Camera) -> Self {
        let dir = (camera.target - camera.position).normalize_or_zero();
        let pitch = dir.y.clamp(-1.0, 1.0).asin();
        let yaw = dir.x.atan2(-dir.z);
        Self {
            position: camera.position,
            yaw,
            pitch,
            ..Default::default()
        }
    }
}

/// Distance from which a sphere of `radius` exactly fills the vertical FOV.
pub fn framing_distance(radius: f32, fov_y_radians: f32) -> f32 {
    (radius / (fov_y_radians * 0.5).sin().max(1e-4)).max(radius)
}

/// Point an orbit controller at a bounding sphere so it fills the view. Sets the
/// target to `center` and the distance to fit `radius` for the camera's FOV.
pub fn frame_orbit(orbit: &mut OrbitController, camera: &Camera, center: Vec3, radius: f32) {
    orbit.target = center;
    let fov = match camera.projection {
        Projection::Perspective { fov_y_radians, .. } => fov_y_radians,
        Projection::Orthographic { .. } => std::f32::consts::FRAC_PI_4,
    };
    orbit.distance = framing_distance(radius.max(1e-4), fov).clamp(orbit.min_distance, orbit.max_distance);
}

/// Move a camera straight back from `center` along its current view direction so
/// a sphere of `radius` fits the frame, keeping the look direction.
pub fn frame_camera(camera: &mut Camera, center: Vec3, radius: f32) {
    let fov = match camera.projection {
        Projection::Perspective { fov_y_radians, .. } => fov_y_radians,
        Projection::Orthographic { .. } => std::f32::consts::FRAC_PI_4,
    };
    let dist = framing_distance(radius.max(1e-4), fov);
    let dir = (camera.position - center).normalize_or_zero();
    let dir = if dir.length_squared() < 1e-6 { Vec3::Z } else { dir };
    camera.target = center;
    camera.position = center + dir * dist;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fly_forward_default_is_minus_z() {
        let fly = FlyController::default();
        assert!((fly.forward() - Vec3::NEG_Z).length() < 1e-5);
    }

    #[test]
    fn move_forward_advances_along_view() {
        let mut fly = FlyController::default();
        let start = fly.position;
        fly.move_local(Vec3::new(0.0, 0.0, 1.0), 1.0);
        // Moves toward -Z (forward), 5 units at default speed.
        assert!((fly.position.z - (start.z - 5.0)).abs() < 1e-4);
    }

    #[test]
    fn look_clamps_pitch() {
        let mut fly = FlyController::default();
        fly.look(0.0, -100000.0); // look way up
        assert!(fly.pitch <= fly.max_pitch);
        fly.look(0.0, 100000.0); // look way down
        assert!(fly.pitch >= fly.min_pitch);
    }

    #[test]
    fn apply_to_sets_target_ahead() {
        let mut fly = FlyController::default();
        fly.yaw = 1.0;
        let mut cam = Camera::perspective(Vec3::ZERO, Vec3::ZERO, 45.0, 1.0);
        fly.apply_to(&mut cam);
        let dir = (cam.target - cam.position).normalize();
        assert!((dir - fly.forward()).length() < 1e-5);
    }

    #[test]
    fn from_camera_roundtrips_direction() {
        let cam = Camera::perspective(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 45.0, 1.0);
        let fly = FlyController::from_camera(&cam);
        assert!((fly.forward() - Vec3::NEG_Z).length() < 1e-4);
    }

    #[test]
    fn frame_orbit_sets_target_and_distance() {
        let cam = Camera::perspective(Vec3::ZERO, Vec3::ZERO, 45.0, 1.0);
        let mut orbit = OrbitController::default();
        frame_orbit(&mut orbit, &cam, Vec3::new(1.0, 2.0, 3.0), 2.0);
        assert_eq!(orbit.target, Vec3::new(1.0, 2.0, 3.0));
        // Distance must be at least the radius and finite.
        assert!(orbit.distance >= 2.0);
    }

    #[test]
    fn frame_camera_keeps_direction_and_fits() {
        let mut cam = Camera::perspective(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 45.0, 1.0);
        frame_camera(&mut cam, Vec3::ZERO, 1.0);
        // Still looking down -Z, just repositioned.
        assert_eq!(cam.target, Vec3::ZERO);
        assert!(cam.position.z > 0.0);
    }
}
