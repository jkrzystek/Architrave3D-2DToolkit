//! Small ray/line/plane helpers used by gizmo interaction. Renderer-agnostic.

use glam::Vec3;
use toolkit_geometry::Ray;

/// Parameter `s` of the point on the infinite line `point + s*dir` closest to
/// the ray. Returns `None` when the ray and line are (near) parallel.
pub fn closest_param_on_line(ray: &Ray, point: Vec3, dir: Vec3) -> Option<f32> {
    let r = ray.direction;
    let d = dir;
    let w0 = ray.origin - point;
    let a = r.dot(r);
    let b = r.dot(d);
    let c = d.dot(d);
    let rd = r.dot(w0);
    let dd = d.dot(w0);
    let denom = a * c - b * b;
    if denom.abs() < 1e-9 {
        return None;
    }
    Some((a * dd - b * rd) / denom)
}

/// The closest world-space point on the infinite axis line to the ray.
pub fn closest_point_on_line(ray: &Ray, point: Vec3, dir: Vec3) -> Option<Vec3> {
    closest_param_on_line(ray, point, dir).map(|s| point + dir * s)
}

/// Shortest distance between the ray and the infinite axis line.
pub fn ray_line_distance(ray: &Ray, point: Vec3, dir: Vec3) -> f32 {
    match closest_point_on_line(ray, point, dir) {
        Some(on_line) => {
            // Distance from that point to the ray.
            let w = on_line - ray.origin;
            let proj = w.dot(ray.direction);
            let closest_on_ray = ray.origin + ray.direction * proj.max(0.0);
            (on_line - closest_on_ray).length()
        }
        None => {
            // Parallel: perpendicular component of (origin - point).
            let w = ray.origin - point;
            (w - dir * w.dot(dir) / dir.dot(dir)).length()
        }
    }
}

/// Intersect the ray with the plane through `point` with normal `normal`.
/// Returns the world-space hit, or `None` if parallel / behind the origin.
pub fn ray_plane_intersection(ray: &Ray, point: Vec3, normal: Vec3) -> Option<Vec3> {
    let denom = ray.direction.dot(normal);
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = (point - ray.origin).dot(normal) / denom;
    if t < 0.0 {
        return None;
    }
    Some(ray.at(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closest_param_on_x_axis() {
        let ray = Ray::new(Vec3::new(0.5, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let s = closest_param_on_line(&ray, Vec3::ZERO, Vec3::X).unwrap();
        assert!((s - 0.5).abs() < 1e-5);
    }

    #[test]
    fn ray_line_distance_zero_when_intersecting() {
        let ray = Ray::new(Vec3::new(0.5, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let dist = ray_line_distance(&ray, Vec3::ZERO, Vec3::X);
        assert!(dist < 1e-4, "dist {dist}");
    }

    #[test]
    fn ray_line_distance_offset() {
        // Ray going down at z=2 is 2 units from the X axis.
        let ray = Ray::new(Vec3::new(0.5, 5.0, 2.0), Vec3::new(0.0, -1.0, 0.0));
        let dist = ray_line_distance(&ray, Vec3::ZERO, Vec3::X);
        assert!((dist - 2.0).abs() < 1e-4, "dist {dist}");
    }

    #[test]
    fn plane_intersection_hits_xz() {
        let ray = Ray::new(Vec3::new(1.0, 5.0, 3.0), Vec3::new(0.0, -1.0, 0.0));
        let hit = ray_plane_intersection(&ray, Vec3::ZERO, Vec3::Y).unwrap();
        assert!((hit - Vec3::new(1.0, 0.0, 3.0)).length() < 1e-4);
    }
}
