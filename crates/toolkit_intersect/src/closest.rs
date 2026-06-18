//! Closest-point queries. These underpin the distance-based intersection
//! tests (a capsule test is just "are the core segments closer than the radius
//! sum") and are useful on their own for snapping and picking.

use glam::Vec3;
use toolkit_geometry::Aabb;

use crate::shapes::{Plane, Segment};

/// Closest point on segment `seg` to `p`, plus the clamped parameter `t` in
/// `[0, 1]` along the segment.
pub fn closest_point_on_segment(seg: Segment, p: Vec3) -> (Vec3, f32) {
    let ab = seg.b - seg.a;
    let len_sq = ab.length_squared();
    if len_sq <= f32::EPSILON {
        return (seg.a, 0.0);
    }
    let t = ((p - seg.a).dot(ab) / len_sq).clamp(0.0, 1.0);
    (seg.a + ab * t, t)
}

/// Closest point on (or inside) an AABB to `p`. Returns `p` itself if inside.
pub fn closest_point_on_aabb(aabb: &Aabb, p: Vec3) -> Vec3 {
    p.clamp(aabb.min, aabb.max)
}

/// Orthogonal projection of `p` onto a plane.
pub fn closest_point_on_plane(plane: &Plane, p: Vec3) -> Vec3 {
    p - plane.normal * plane.signed_distance(p)
}

/// Closest points between two segments, returned as `(point_on_s1,
/// point_on_s2)`. Handles parallel and degenerate (point) segments.
///
/// Based on Ericson, *Real-Time Collision Detection*, §5.1.9.
pub fn closest_points_between_segments(s1: Segment, s2: Segment) -> (Vec3, Vec3) {
    let d1 = s1.b - s1.a; // direction of s1
    let d2 = s2.b - s2.a; // direction of s2
    let r = s1.a - s2.a;
    let a = d1.length_squared();
    let e = d2.length_squared();
    let f = d2.dot(r);

    const EPS: f32 = 1e-9;

    // Both segments degenerate to points.
    if a <= EPS && e <= EPS {
        return (s1.a, s2.a);
    }

    let (s, t);
    if a <= EPS {
        // First segment is a point.
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= EPS {
            // Second segment is a point.
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            // If not parallel, compute closest point on L1 to L2; else pick 0.
            let s_tmp = if denom > EPS {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            // Compute t for that s, then clamp and recompute s if needed.
            let t_tmp = (b * s_tmp + f) / e;
            if t_tmp < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t_tmp > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            } else {
                t = t_tmp;
                s = s_tmp;
            }
        }
    }

    (s1.a + d1 * s, s2.a + d2 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_geometry::Aabb;

    #[test]
    fn point_projects_onto_segment_interior() {
        let seg = Segment::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let (cp, t) = closest_point_on_segment(seg, Vec3::new(3.0, 5.0, 0.0));
        assert!((cp - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
        assert!((t - 0.3).abs() < 1e-5);
    }

    #[test]
    fn point_clamps_to_segment_end() {
        let seg = Segment::new(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let (cp, t) = closest_point_on_segment(seg, Vec3::new(-5.0, 1.0, 0.0));
        assert_eq!(cp, Vec3::ZERO);
        assert_eq!(t, 0.0);
    }

    #[test]
    fn closest_on_aabb_clamps() {
        let aabb = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let cp = closest_point_on_aabb(&aabb, Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(cp, Vec3::new(1.0, 0.0, 0.0));
        // Inside point returns itself.
        assert_eq!(closest_point_on_aabb(&aabb, Vec3::ZERO), Vec3::ZERO);
    }

    #[test]
    fn closest_on_plane_projects() {
        let plane = Plane::from_point_normal(Vec3::ZERO, Vec3::Y);
        let cp = closest_point_on_plane(&plane, Vec3::new(2.0, 7.0, -3.0));
        assert!((cp - Vec3::new(2.0, 0.0, -3.0)).length() < 1e-5);
    }

    #[test]
    fn crossing_segments_closest_at_crossing() {
        let s1 = Segment::new(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        let s2 = Segment::new(Vec3::new(0.0, -1.0, 1.0), Vec3::new(0.0, 1.0, 1.0));
        let (p1, p2) = closest_points_between_segments(s1, s2);
        assert!((p1 - Vec3::ZERO).length() < 1e-5);
        assert!((p2 - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-5);
    }

    #[test]
    fn parallel_segments() {
        let s1 = Segment::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 0.0));
        let s2 = Segment::new(Vec3::new(1.0, 2.0, 0.0), Vec3::new(5.0, 2.0, 0.0));
        let (p1, p2) = closest_points_between_segments(s1, s2);
        // Distance between the lines is 2 along Y.
        assert!(((p2 - p1).length() - 2.0).abs() < 1e-4);
    }
}
