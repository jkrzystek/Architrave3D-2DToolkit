use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ViewUniforms {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
    pub view_projection: [[f32; 4]; 4],
    pub inverse_view_projection: [[f32; 4]; 4],
    pub camera_position: [f32; 4],
    pub viewport_size: [f32; 2],
    pub near_plane: f32,
    pub far_plane: f32,
}

impl ViewUniforms {
    pub fn from_camera(cam: &super::camera::Camera, viewport_width: f32, viewport_height: f32) -> Self {
        let view = cam.view_matrix();
        let proj = cam.projection_matrix();
        let vp = proj * view;
        let inv_vp = vp.inverse();

        let (near, far) = match cam.projection {
            super::camera::Projection::Perspective { near, far, .. } => (near, far),
            super::camera::Projection::Orthographic { near, far, .. } => (near, far),
        };

        Self {
            view: view.to_cols_array_2d(),
            projection: proj.to_cols_array_2d(),
            view_projection: vp.to_cols_array_2d(),
            inverse_view_projection: inv_vp.to_cols_array_2d(),
            camera_position: Vec4::new(cam.position.x, cam.position.y, cam.position.z, 1.0)
                .to_array(),
            viewport_size: [viewport_width, viewport_height],
            near_plane: near,
            far_plane: far,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ModelUniforms {
    pub model: [[f32; 4]; 4],
    pub normal_matrix: [[f32; 4]; 4],
}

impl ModelUniforms {
    pub fn from_transform(model: Mat4) -> Self {
        let normal = model.inverse().transpose();
        Self {
            model: model.to_cols_array_2d(),
            normal_matrix: normal.to_cols_array_2d(),
        }
    }

    pub fn identity() -> Self {
        Self::from_transform(Mat4::IDENTITY)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightUniforms {
    pub direction: [f32; 4],
    pub color: [f32; 4],
    pub ambient_color: [f32; 4],
    pub intensity: f32,
    pub ambient_intensity: f32,
    pub _padding: [f32; 2],
}

impl Default for LightUniforms {
    fn default() -> Self {
        let dir = Vec3::new(-0.5, -1.0, -0.3).normalize();
        Self {
            direction: [dir.x, dir.y, dir.z, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
            ambient_color: [0.1, 0.1, 0.15, 1.0],
            intensity: 1.0,
            ambient_intensity: 0.3,
            _padding: [0.0; 2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;

    #[test]
    fn view_uniforms_from_camera() {
        let cam = Camera::perspective(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            45.0,
            16.0 / 9.0,
        );
        let uniforms = ViewUniforms::from_camera(&cam, 1920.0, 1080.0);
        assert_eq!(uniforms.viewport_size, [1920.0, 1080.0]);
        assert!((uniforms.camera_position[2] - 5.0).abs() < 1e-5);
    }

    #[test]
    fn model_uniforms_identity() {
        let u = ModelUniforms::identity();
        let m = Mat4::from_cols_array_2d(&u.model);
        assert!((m - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-6));
    }

    #[test]
    fn light_uniforms_normalized_direction() {
        let l = LightUniforms::default();
        let dir = Vec3::new(l.direction[0], l.direction[1], l.direction[2]);
        assert!((dir.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn uniform_sizes_are_pod() {
        assert_eq!(std::mem::size_of::<ViewUniforms>() % 16, 0);
        assert_eq!(std::mem::size_of::<ModelUniforms>() % 16, 0);
        assert_eq!(std::mem::size_of::<LightUniforms>() % 16, 0);
    }
}
