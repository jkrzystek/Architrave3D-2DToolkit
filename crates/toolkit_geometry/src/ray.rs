use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::bvh::{Bvh, BvhNode};
use crate::mesh::{Aabb, Mesh};

// ---------------------------------------------------------------------------
// Ray
// ---------------------------------------------------------------------------

/// A ray defined by an origin and a *normalised* direction.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    /// Create a ray; the direction is normalised automatically.
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Return the point at parameter `t` along the ray.
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

// ---------------------------------------------------------------------------
// HitRecord
// ---------------------------------------------------------------------------

/// Information about a ray/mesh intersection.
#[derive(Clone, Copy, Debug)]
pub struct HitRecord {
    /// Distance along the ray to the hit point.
    pub t: f32,
    /// World-space position of the hit.
    pub position: Vec3,
    /// Interpolated surface normal at the hit.
    pub normal: Vec3,
    /// Interpolated UV coordinates at the hit.
    pub uv: Vec2,
    /// Index of the triangle that was hit (in units of triangles, not indices).
    pub triangle_index: u32,
}

// ---------------------------------------------------------------------------
// Moller-Trumbore ray-triangle intersection
// ---------------------------------------------------------------------------

/// Moller-Trumbore ray-triangle intersection.
///
/// Returns `Some((t, u, v))` where `t` is the distance along the ray and
/// `(u, v)` are the barycentric coordinates on the triangle (the third
/// coordinate `w = 1 - u - v`).
///
/// Returns `None` if the ray misses or the triangle is degenerate.
pub fn ray_triangle_intersection(
    ray: &Ray,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<(f32, f32, f32)> {
    const EPSILON: f32 = 1e-7;

    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray.direction.cross(edge2);
    let a = edge1.dot(h);

    // Ray is parallel to the triangle.
    if a.abs() < EPSILON {
        return None;
    }

    let f = 1.0 / a;
    let s = ray.origin - v0;
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray.direction.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);
    if t > EPSILON {
        Some((t, u, v))
    } else {
        None // Intersection is behind the ray origin.
    }
}

// ---------------------------------------------------------------------------
// Slab-method ray-AABB intersection
// ---------------------------------------------------------------------------

/// Slab-method ray-AABB test.
///
/// Returns `Some((t_near, t_far))` if the ray intersects the box, with
/// `t_near <= t_far`. A negative `t_near` indicates the ray origin is inside
/// the box.
pub fn ray_aabb_intersection(ray: &Ray, aabb: &Aabb) -> Option<(f32, f32)> {
    let inv_dir = Vec3::new(
        safe_inv(ray.direction.x),
        safe_inv(ray.direction.y),
        safe_inv(ray.direction.z),
    );

    let t1 = (aabb.min - ray.origin) * inv_dir;
    let t2 = (aabb.max - ray.origin) * inv_dir;

    let t_min_v = t1.min(t2);
    let t_max_v = t1.max(t2);

    let t_near = t_min_v.x.max(t_min_v.y).max(t_min_v.z);
    let t_far = t_max_v.x.min(t_max_v.y).min(t_max_v.z);

    if t_near <= t_far && t_far >= 0.0 {
        Some((t_near, t_far))
    } else {
        None
    }
}

/// Safe reciprocal that avoids division by zero by returning a large value.
#[inline]
fn safe_inv(x: f32) -> f32 {
    if x.abs() < 1e-30 {
        x.signum() * 1e30
    } else {
        1.0 / x
    }
}

// ---------------------------------------------------------------------------
// BVH traversal
// ---------------------------------------------------------------------------

impl Bvh {
    /// Find the closest ray intersection in the mesh using BVH traversal.
    pub fn intersect(&self, ray: &Ray, mesh: &Mesh) -> Option<HitRecord> {
        let mut closest: Option<HitRecord> = None;
        let mut t_max = f32::INFINITY;
        intersect_recursive(ray, mesh, &self.root, &mut closest, &mut t_max);
        closest
    }
}

fn intersect_recursive(
    ray: &Ray,
    mesh: &Mesh,
    node: &BvhNode,
    closest: &mut Option<HitRecord>,
    t_max: &mut f32,
) {
    // Early-out if the ray misses this node's AABB.
    match ray_aabb_intersection(ray, node.aabb()) {
        Some((_t_near, t_far)) => {
            if t_far < 0.0 || _t_near > *t_max {
                return;
            }
        }
        None => return,
    }

    match node {
        BvhNode::Leaf {
            triangle_indices, ..
        } => {
            for &tri_idx in triangle_indices {
                let base = tri_idx as usize * 3;
                let i0 = mesh.indices[base] as usize;
                let i1 = mesh.indices[base + 1] as usize;
                let i2 = mesh.indices[base + 2] as usize;

                let v0 = &mesh.vertices[i0];
                let v1 = &mesh.vertices[i1];
                let v2 = &mesh.vertices[i2];

                if let Some((t, u, v)) =
                    ray_triangle_intersection(ray, v0.position_vec3(), v1.position_vec3(), v2.position_vec3())
                {
                    if t < *t_max {
                        let w = 1.0 - u - v;
                        let normal = (w * v0.normal_vec3() + u * v1.normal_vec3() + v * v2.normal_vec3()).normalize();
                        let uv_interp = w * v0.uv_vec2() + u * v1.uv_vec2() + v * v2.uv_vec2();

                        *t_max = t;
                        *closest = Some(HitRecord {
                            t,
                            position: ray.at(t),
                            normal,
                            uv: uv_interp,
                            triangle_index: tri_idx,
                        });
                    }
                }
            }
        }
        BvhNode::Internal { left, right, .. } => {
            // Traverse both children; no ordering optimisation (simple version).
            intersect_recursive(ray, mesh, left, closest, t_max);
            intersect_recursive(ray, mesh, right, closest, t_max);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Ray-triangle --------------------------------------------------------

    #[test]
    fn ray_hits_triangle() {
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        let ray = Ray::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = ray_triangle_intersection(&ray, v0, v1, v2);
        assert!(hit.is_some());
        let (t, u, v) = hit.unwrap();
        assert!((t - 1.0).abs() < 1e-4);
        // Origin projects to centroid-ish of this triangle.
        assert!(u >= 0.0 && v >= 0.0 && u + v <= 1.0);
    }

    #[test]
    fn ray_misses_triangle() {
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        // Shoot to the side.
        let ray = Ray::new(Vec3::new(5.0, 5.0, 1.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(ray_triangle_intersection(&ray, v0, v1, v2).is_none());
    }

    #[test]
    fn ray_behind_origin_no_hit() {
        let v0 = Vec3::new(-1.0, -1.0, 0.0);
        let v1 = Vec3::new(1.0, -1.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        // Facing away from the triangle.
        let ray = Ray::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(ray_triangle_intersection(&ray, v0, v1, v2).is_none());
    }

    // -- Ray-AABB ------------------------------------------------------------

    #[test]
    fn ray_hits_aabb() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::ONE);
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = ray_aabb_intersection(&ray, &aabb);
        assert!(hit.is_some());
        let (t_near, t_far) = hit.unwrap();
        assert!((t_near - 4.0).abs() < 1e-4);
        assert!((t_far - 6.0).abs() < 1e-4);
    }

    #[test]
    fn ray_misses_aabb() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::ONE);
        let ray = Ray::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(ray_aabb_intersection(&ray, &aabb).is_none());
    }

    #[test]
    fn ray_origin_inside_aabb() {
        let aabb = Aabb::new(Vec3::new(-2.0, -2.0, -2.0), Vec3::splat(2.0));
        let ray = Ray::new(Vec3::ZERO, Vec3::X);
        let hit = ray_aabb_intersection(&ray, &aabb);
        assert!(hit.is_some());
        let (t_near, _t_far) = hit.unwrap();
        // t_near should be negative (origin inside).
        assert!(t_near <= 0.0);
    }

    #[test]
    fn ray_parallel_to_face() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::ONE);
        // Ray parallel to XZ plane at y=0, should pass through the box.
        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);
        let hit = ray_aabb_intersection(&ray, &aabb);
        assert!(hit.is_some());
    }

    #[test]
    fn ray_parallel_outside() {
        let aabb = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::ONE);
        // Parallel to X axis but outside the box on Y.
        let ray = Ray::new(Vec3::new(-5.0, 3.0, 0.0), Vec3::X);
        assert!(ray_aabb_intersection(&ray, &aabb).is_none());
    }

    // -- BVH intersection ----------------------------------------------------

    #[test]
    fn bvh_ray_hits_cube() {
        let cube = Mesh::cube(2.0);
        let bvh = Bvh::build(&cube);

        // Shoot a ray from outside toward the center.
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = bvh.intersect(&ray, &cube);
        assert!(hit.is_some());
        let hit = hit.unwrap();
        // Should hit the +Z face at z=1.
        assert!((hit.position.z - 1.0).abs() < 1e-3);
        // Normal should point toward the ray (outward).
        assert!(hit.normal.z > 0.5);
    }

    #[test]
    fn bvh_ray_misses_cube() {
        let cube = Mesh::cube(2.0);
        let bvh = Bvh::build(&cube);

        // Shoot a ray that misses entirely.
        let ray = Ray::new(Vec3::new(10.0, 10.0, 10.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(bvh.intersect(&ray, &cube).is_none());
    }

    #[test]
    fn bvh_ray_hits_closest_face() {
        let cube = Mesh::cube(2.0);
        let bvh = Bvh::build(&cube);

        // Shoot along -Y from above.
        let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let hit = bvh.intersect(&ray, &cube).unwrap();
        // Should hit the +Y face at y=1.
        assert!((hit.position.y - 1.0).abs() < 1e-3);
    }

    #[test]
    fn ray_at_returns_correct_point() {
        let ray = Ray::new(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
        let p = ray.at(3.0);
        assert!((p - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
    }
}
