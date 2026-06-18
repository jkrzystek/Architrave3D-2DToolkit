//! Boolean overlap tests between bounding shapes. Most reduce to a
//! closest-point distance compared against a radius.

use glam::Vec3;
use toolkit_geometry::Aabb;

use crate::closest::{
    closest_point_on_aabb, closest_points_between_segments, closest_point_on_segment,
};
use crate::shapes::{Capsule, Plane, Segment, Sphere};

/// Two spheres overlap if their centres are within the sum of radii.
pub fn sphere_sphere(a: &Sphere, b: &Sphere) -> bool {
    let r = a.radius + b.radius;
    (a.center - b.center).length_squared() <= r * r
}

/// A sphere overlaps an AABB if the box's closest point is within the radius.
pub fn sphere_aabb(s: &Sphere, aabb: &Aabb) -> bool {
    let cp = closest_point_on_aabb(aabb, s.center);
    (cp - s.center).length_squared() <= s.radius * s.radius
}

/// A sphere intersects a plane if the centre is within `radius` of it.
pub fn sphere_plane(s: &Sphere, plane: &Plane) -> bool {
    plane.signed_distance(s.center).abs() <= s.radius
}

/// Shortest distance from a segment to a sphere's surface; `<= 0` means the
/// segment touches or pierces the sphere. Returns the gap (negative if
/// overlapping).
pub fn segment_sphere_gap(seg: Segment, s: &Sphere) -> f32 {
    let (cp, _) = closest_point_on_segment(seg, s.center);
    (cp - s.center).length() - s.radius
}

/// Whether a segment touches a sphere.
pub fn segment_sphere(seg: Segment, s: &Sphere) -> bool {
    segment_sphere_gap(seg, s) <= 0.0
}

/// Two capsules overlap if their core segments are closer than the radius sum.
pub fn capsule_capsule(a: &Capsule, b: &Capsule) -> bool {
    let (p1, p2) = closest_points_between_segments(a.segment(), b.segment());
    let r = a.radius + b.radius;
    (p1 - p2).length_squared() <= r * r
}

/// Classify an AABB against a plane: `1` fully in front (normal side), `-1`
/// fully behind, `0` straddling.
pub fn aabb_plane_side(aabb: &Aabb, plane: &Plane) -> i32 {
    // Project the box half-extents onto the plane normal.
    let center = aabb.center();
    let extents = aabb.half_extents();
    let radius = extents.x * plane.normal.x.abs()
        + extents.y * plane.normal.y.abs()
        + extents.z * plane.normal.z.abs();
    let dist = plane.signed_distance(center);
    if dist > radius {
        1
    } else if dist < -radius {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spheres_overlap_and_not() {
        let a = Sphere::new(Vec3::ZERO, 1.0);
        let b = Sphere::new(Vec3::new(1.5, 0.0, 0.0), 1.0);
        assert!(sphere_sphere(&a, &b));
        let c = Sphere::new(Vec3::new(5.0, 0.0, 0.0), 1.0);
        assert!(!sphere_sphere(&a, &c));
    }

    #[test]
    fn sphere_vs_aabb() {
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        assert!(sphere_aabb(&Sphere::new(Vec3::new(1.5, 0.0, 0.0), 0.6), &aabb));
        assert!(!sphere_aabb(&Sphere::new(Vec3::new(3.0, 0.0, 0.0), 0.6), &aabb));
    }

    #[test]
    fn sphere_vs_plane() {
        let plane = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        assert!(sphere_plane(&Sphere::new(Vec3::new(0.0, 0.4, 0.0), 0.5), &plane));
        assert!(!sphere_plane(&Sphere::new(Vec3::new(0.0, 2.0, 0.0), 0.5), &plane));
    }

    #[test]
    fn segment_vs_sphere() {
        let s = Sphere::new(Vec3::ZERO, 1.0);
        let hit = Segment::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0));
        assert!(segment_sphere(hit, &s));
        let miss = Segment::new(Vec3::new(-5.0, 3.0, 0.0), Vec3::new(5.0, 3.0, 0.0));
        assert!(!segment_sphere(miss, &s));
    }

    #[test]
    fn capsules_overlap() {
        let a = Capsule::new(Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0), 0.5);
        let b = Capsule::new(Vec3::new(0.0, 0.8, 0.0), Vec3::new(0.0, 4.0, 0.0), 0.5);
        assert!(capsule_capsule(&a, &b));
        let c = Capsule::new(Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.0, 5.0, 0.0), 0.5);
        assert!(!capsule_capsule(&a, &c));
    }

    #[test]
    fn aabb_plane_classification() {
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let above = Plane::from_point_normal(Vec3::new(0.0, 5.0, 0.0), Vec3::Y);
        assert_eq!(aabb_plane_side(&aabb, &above), -1);
        let below = Plane::from_point_normal(Vec3::new(0.0, -5.0, 0.0), Vec3::Y);
        assert_eq!(aabb_plane_side(&aabb, &below), 1);
        let through = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        assert_eq!(aabb_plane_side(&aabb, &through), 0);
    }
}
