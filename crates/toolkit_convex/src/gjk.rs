//! GJK distance between two convex point sets.
//!
//! GJK searches the Minkowski difference `A ⊖ B`: the two shapes are disjoint
//! iff it excludes the origin, and their separation equals the origin's
//! distance to it. Each iteration keeps the simplex (≤ 4 Minkowski points)
//! closest to the origin and walks toward the origin via support points.

use glam::Vec3;

/// Farthest point of a convex point set along `dir`.
fn support(points: &[Vec3], dir: Vec3) -> Vec3 {
    *points
        .iter()
        .max_by(|a, b| a.dot(dir).total_cmp(&b.dot(dir)))
        .expect("support called on empty point set")
}

/// Minkowski-difference support point of `a ⊖ b` along `dir`.
fn minkowski(a: &[Vec3], b: &[Vec3], dir: Vec3) -> Vec3 {
    support(a, dir) - support(b, -dir)
}

/// Distance between the convex hulls of two point sets. Returns `0.0` if they
/// overlap. Both sets must be non-empty.
pub fn gjk_distance(a: &[Vec3], b: &[Vec3]) -> f32 {
    assert!(!a.is_empty() && !b.is_empty(), "gjk_distance needs non-empty sets");

    let mut simplex: Vec<Vec3> = vec![minkowski(a, b, Vec3::X)];
    let tol = 1e-10;

    for _ in 0..64 {
        let (closest, reduced) = closest_to_origin(&simplex);
        simplex = reduced;

        let dist_sq = closest.length_squared();
        if dist_sq <= tol {
            return 0.0; // origin inside the simplex → shapes overlap
        }

        // Search toward the origin.
        let dir = -closest;
        let w = minkowski(a, b, dir);

        // Convergence: no support point lies appreciably closer to the origin.
        let progress = closest.dot(closest) - closest.dot(w);
        if progress <= tol * dist_sq {
            return dist_sq.sqrt();
        }
        // Avoid re-adding an existing vertex (numerical stall).
        if simplex.iter().any(|&s| (s - w).length_squared() <= tol) {
            return dist_sq.sqrt();
        }
        simplex.push(w);
    }

    // Fallback after the iteration cap: report the best estimate.
    closest_to_origin(&simplex).0.length()
}

/// Whether the two convex sets overlap (distance ~ 0).
pub fn hulls_intersect(a: &[Vec3], b: &[Vec3]) -> bool {
    gjk_distance(a, b) <= 1e-5
}

/// Closest point on a simplex (1–4 points) to the origin, plus the reduced
/// simplex (the sub-feature actually containing that closest point).
fn closest_to_origin(simplex: &[Vec3]) -> (Vec3, Vec<Vec3>) {
    match simplex.len() {
        1 => (simplex[0], vec![simplex[0]]),
        2 => closest_on_segment(simplex[0], simplex[1]),
        3 => closest_on_triangle(simplex[0], simplex[1], simplex[2]),
        _ => closest_on_tetra(simplex[0], simplex[1], simplex[2], simplex[3]),
    }
}

const O: Vec3 = Vec3::ZERO;

fn closest_on_segment(a: Vec3, b: Vec3) -> (Vec3, Vec<Vec3>) {
    let ab = b - a;
    let denom = ab.length_squared();
    if denom <= 1e-20 {
        return (a, vec![a]);
    }
    let t = (O - a).dot(ab) / denom;
    if t <= 0.0 {
        (a, vec![a])
    } else if t >= 1.0 {
        (b, vec![b])
    } else {
        (a + ab * t, vec![a, b])
    }
}

fn closest_on_triangle(a: Vec3, b: Vec3, c: Vec3) -> (Vec3, Vec<Vec3>) {
    // Ericson, Real-Time Collision Detection §5.1.5, with origin as the query.
    let ab = b - a;
    let ac = c - a;
    let ap = O - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, vec![a]);
    }
    let bp = O - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, vec![b]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let t = d1 / (d1 - d3);
        return (a + ab * t, vec![a, b]);
    }
    let cp = O - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, vec![c]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let t = d2 / (d2 - d6);
        return (a + ac * t, vec![a, c]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let t = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * t, vec![b, c]);
    }
    // Interior of the face.
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (a + ab * v + ac * w, vec![a, b, c])
}

fn closest_on_tetra(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> (Vec3, Vec<Vec3>) {
    let mut best_point = O;
    let mut best_sq = f32::INFINITY;
    let mut best_set: Vec<Vec3> = vec![a, b, c, d];
    let mut outside_any = false;

    // Each face with the opposite vertex (for outside-ness orientation).
    let faces = [
        ([a, b, c], d),
        ([a, c, d], b),
        ([a, d, b], c),
        ([b, d, c], a),
    ];
    for (tri, opp) in faces {
        if point_outside_plane(tri[0], tri[1], tri[2], opp) {
            outside_any = true;
            let (q, set) = closest_on_triangle(tri[0], tri[1], tri[2]);
            let sq = q.length_squared();
            if sq < best_sq {
                best_sq = sq;
                best_point = q;
                best_set = set;
            }
        }
    }

    if !outside_any {
        // Origin is inside the tetrahedron.
        (O, vec![a, b, c, d])
    } else {
        (best_point, best_set)
    }
}

/// Is the origin on the opposite side of plane `(a, b, c)` from `opp`?
fn point_outside_plane(a: Vec3, b: Vec3, c: Vec3, opp: Vec3) -> bool {
    let n = (b - a).cross(c - a);
    let sign_o = n.dot(O - a);
    let sign_opp = n.dot(opp - a);
    // Outside when the origin and the opposite vertex are on different sides.
    sign_o * sign_opp < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(center: Vec3, half: f32) -> Vec<Vec3> {
        let mut v = Vec::new();
        for &sx in &[-1.0f32, 1.0] {
            for &sy in &[-1.0f32, 1.0] {
                for &sz in &[-1.0f32, 1.0] {
                    v.push(center + Vec3::new(sx, sy, sz) * half);
                }
            }
        }
        v
    }

    #[test]
    fn separated_cubes_distance() {
        let a = cube(Vec3::ZERO, 1.0); // spans x in [-1, 1]
        let b = cube(Vec3::new(5.0, 0.0, 0.0), 1.0); // spans x in [4, 6]
        // Gap along x is 4 - 1 = 3.
        assert!((gjk_distance(&a, &b) - 3.0).abs() < 1e-3);
    }

    #[test]
    fn overlapping_cubes_zero_distance() {
        let a = cube(Vec3::ZERO, 1.0);
        let b = cube(Vec3::new(0.5, 0.0, 0.0), 1.0);
        assert_eq!(gjk_distance(&a, &b), 0.0);
        assert!(hulls_intersect(&a, &b));
    }

    #[test]
    fn touching_cubes_near_zero() {
        let a = cube(Vec3::ZERO, 1.0); // x in [-1, 1]
        let b = cube(Vec3::new(2.0, 0.0, 0.0), 1.0); // x in [1, 3], faces touch
        assert!(gjk_distance(&a, &b) < 1e-3);
    }

    #[test]
    fn diagonal_separation() {
        let a = cube(Vec3::ZERO, 1.0);
        let b = cube(Vec3::new(3.0, 3.0, 3.0), 1.0);
        // Closest corners: (1,1,1) and (2,2,2), distance sqrt(3).
        let expected = (3.0f32).sqrt();
        assert!((gjk_distance(&a, &b) - expected).abs() < 1e-2);
    }

    #[test]
    fn points_distance() {
        let a = vec![Vec3::ZERO];
        let b = vec![Vec3::new(3.0, 4.0, 0.0)];
        assert!((gjk_distance(&a, &b) - 5.0).abs() < 1e-4);
    }

    #[test]
    fn not_intersecting_when_apart() {
        let a = cube(Vec3::ZERO, 1.0);
        let b = cube(Vec3::new(10.0, 0.0, 0.0), 1.0);
        assert!(!hulls_intersect(&a, &b));
    }
}
