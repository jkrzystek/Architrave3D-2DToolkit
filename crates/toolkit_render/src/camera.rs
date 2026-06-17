use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Projection {
    Perspective {
        fov_y_radians: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    },
    Orthographic {
        width: f32,
        height: f32,
        near: f32,
        far: f32,
    },
}

impl Projection {
    pub fn matrix(&self) -> Mat4 {
        match *self {
            Self::Perspective {
                fov_y_radians,
                aspect_ratio,
                near,
                far,
            } => Mat4::perspective_rh(fov_y_radians, aspect_ratio, near, far),
            Self::Orthographic {
                width,
                height,
                near,
                far,
            } => Mat4::orthographic_rh(
                -width * 0.5,
                width * 0.5,
                -height * 0.5,
                height * 0.5,
                near,
                far,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub projection: Projection,
}

impl Camera {
    pub fn perspective(position: Vec3, target: Vec3, fov_degrees: f32, aspect: f32) -> Self {
        Self {
            position,
            target,
            up: Vec3::Y,
            projection: Projection::Perspective {
                fov_y_radians: fov_degrees.to_radians(),
                aspect_ratio: aspect,
                near: 0.1,
                far: 1000.0,
            },
        }
    }

    pub fn orthographic(position: Vec3, target: Vec3, width: f32, height: f32) -> Self {
        Self {
            position,
            target,
            up: Vec3::Y,
            projection: Projection::Orthographic {
                width,
                height,
                near: -1000.0,
                far: 1000.0,
            },
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        self.projection.matrix()
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up).normalize()
    }

    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        if let Projection::Perspective {
            ref mut aspect_ratio,
            ..
        } = self.projection
        {
            *aspect_ratio = aspect;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitController {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub target: Vec3,
    pub min_distance: f32,
    pub max_distance: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,
}

impl Default for OrbitController {
    fn default() -> Self {
        Self {
            distance: 5.0,
            yaw: 0.0,
            pitch: 0.3,
            target: Vec3::ZERO,
            min_distance: 0.1,
            max_distance: 1000.0,
            min_pitch: -std::f32::consts::FRAC_PI_2 + 0.01,
            max_pitch: std::f32::consts::FRAC_PI_2 - 0.01,
        }
    }
}

impl OrbitController {
    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(self.min_pitch, self.max_pitch);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance * (1.0 - delta)).clamp(self.min_distance, self.max_distance);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let up = Vec3::Y;
        self.target += right * dx + up * dy;
    }

    pub fn camera_position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + Vec3::new(x, y, z)
    }

    pub fn apply_to(&self, camera: &mut Camera) {
        camera.position = self.camera_position();
        camera.target = self.target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perspective_camera_matrices() {
        let cam = Camera::perspective(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            45.0,
            1.0,
        );
        let view = cam.view_matrix();
        let proj = cam.projection_matrix();
        assert!(view.determinant().abs() > 0.0);
        assert!(proj.determinant().abs() > 0.0);
    }

    #[test]
    fn orthographic_camera() {
        let cam = Camera::orthographic(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 10.0, 10.0);
        let vp = cam.view_projection_matrix();
        assert!(vp.determinant().abs() > 0.0);
    }

    #[test]
    fn camera_forward_direction() {
        let cam = Camera::perspective(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            45.0,
            1.0,
        );
        let fwd = cam.forward();
        assert!((fwd - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-5);
    }

    #[test]
    fn orbit_controller_zoom() {
        let mut orbit = OrbitController::default();
        let initial = orbit.distance;
        orbit.zoom(0.1);
        assert!(orbit.distance < initial);
    }

    #[test]
    fn orbit_controller_pitch_clamped() {
        let mut orbit = OrbitController::default();
        orbit.rotate(0.0, 100.0);
        assert!(orbit.pitch <= orbit.max_pitch);
        orbit.rotate(0.0, -200.0);
        assert!(orbit.pitch >= orbit.min_pitch);
    }

    #[test]
    fn orbit_applies_to_camera() {
        let orbit = OrbitController::default();
        let mut cam = Camera::perspective(Vec3::ZERO, Vec3::ZERO, 45.0, 1.0);
        orbit.apply_to(&mut cam);
        assert_eq!(cam.target, orbit.target);
        let expected_pos = orbit.camera_position();
        assert!((cam.position - expected_pos).length() < 1e-5);
    }
}
