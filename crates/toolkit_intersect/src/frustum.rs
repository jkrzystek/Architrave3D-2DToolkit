//! View frustum extraction and culling.
//!
//! Planes are pulled from a combined view-projection matrix with the
//! Gribb–Hartmann method, using a `[0, 1]` clip-space depth range (the wgpu /
//! Direct3D / Metal / Vulkan convention) rather than OpenGL's `[-1, 1]`.

use glam::{Mat4, Vec3, Vec4};
use toolkit_geometry::Aabb;

use crate::shapes::{Plane, Sphere};

/// Six planes whose inward-pointing normals bound the visible volume.
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    /// Order: left, right, bottom, top, near, far.
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extract the frustum from a `projection * view` matrix.
    pub fn from_view_projection(view_proj: Mat4) -> Self {
        // glam rows give us the linear combinations Gribb–Hartmann needs.
        let r0 = view_proj.row(0);
        let r1 = view_proj.row(1);
        let r2 = view_proj.row(2);
        let r3 = view_proj.row(3);

        let planes = [
            plane_from_vec4(r3 + r0), // left
            plane_from_vec4(r3 - r0), // right
            plane_from_vec4(r3 + r1), // bottom
            plane_from_vec4(r3 - r1), // top
            plane_from_vec4(r2),      // near (z = 0 in [0,1] clip space)
            plane_from_vec4(r3 - r2), // far
        ];
        Self { planes }
    }

    /// `true` if any part of the sphere lies inside the frustum.
    pub fn intersects_sphere(&self, s: &Sphere) -> bool {
        self.planes
            .iter()
            .all(|p| p.signed_distance(s.center) >= -s.radius)
    }

    /// `true` if any part of the AABB lies inside the frustum. Uses the
    /// "positive vertex" test: the box is culled only if it is fully behind
    /// some plane. May report a false positive for boxes straddling a corner,
    /// which is the standard, conservative behaviour for culling.
    pub fn intersects_aabb(&self, aabb: &Aabb) -> bool {
        for p in &self.planes {
            // Farthest corner along the plane normal.
            let positive = Vec3::new(
                if p.normal.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if p.normal.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if p.normal.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );
            if p.signed_distance(positive) < 0.0 {
                return false; // entirely behind this plane → outside
            }
        }
        true
    }

    /// `true` only if the whole sphere is inside (no clipping).
    pub fn contains_sphere(&self, s: &Sphere) -> bool {
        self.planes
            .iter()
            .all(|p| p.signed_distance(s.center) >= s.radius)
    }
}

fn plane_from_vec4(v: Vec4) -> Plane {
    Plane {
        normal: v.truncate(),
        d: v.w,
    }
    .normalized()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Mat4 {
        // Looking down -Z from z=5, 90° fov, square aspect, near 0.1 far 100.
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        proj * view
    }

    #[test]
    fn origin_sphere_is_visible() {
        let f = Frustum::from_view_projection(camera());
        assert!(f.intersects_sphere(&Sphere::new(Vec3::ZERO, 1.0)));
    }

    #[test]
    fn far_behind_camera_is_culled() {
        let f = Frustum::from_view_projection(camera());
        // Well behind the camera (further +Z than the eye).
        assert!(!f.intersects_sphere(&Sphere::new(Vec3::new(0.0, 0.0, 50.0), 1.0)));
    }

    #[test]
    fn far_off_to_the_side_is_culled() {
        let f = Frustum::from_view_projection(camera());
        assert!(!f.intersects_sphere(&Sphere::new(Vec3::new(100.0, 0.0, 0.0), 1.0)));
    }

    #[test]
    fn aabb_at_origin_is_visible() {
        let f = Frustum::from_view_projection(camera());
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        assert!(f.intersects_aabb(&aabb));
    }

    #[test]
    fn aabb_far_to_side_is_culled() {
        let f = Frustum::from_view_projection(camera());
        let aabb = Aabb::new(Vec3::new(99.0, -1.0, -1.0), Vec3::new(101.0, 1.0, 1.0));
        assert!(!f.intersects_aabb(&aabb));
    }

    #[test]
    fn contains_distinguishes_from_intersects() {
        let f = Frustum::from_view_projection(camera());
        // A huge sphere at the origin intersects but is not fully contained.
        let big = Sphere::new(Vec3::ZERO, 10.0);
        assert!(f.intersects_sphere(&big));
        assert!(!f.contains_sphere(&big));
    }
}
