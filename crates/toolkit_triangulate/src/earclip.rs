//! Ear-clipping triangulation of a simple polygon, plus the small geometry
//! predicates shared across the crate.

use glam::Vec2;

/// Signed area of a polygon given as a loop of indices into `points`. Positive
/// for counter-clockwise winding.
pub fn signed_area(points: &[Vec2], loop_idx: &[usize]) -> f32 {
    let n = loop_idx.len();
    let mut a = 0.0;
    for i in 0..n {
        let p = points[loop_idx[i]];
        let q = points[loop_idx[(i + 1) % n]];
        a += p.x * q.y - q.x * p.y;
    }
    a * 0.5
}

/// Twice the signed area of triangle `abc` (cross product); `> 0` if CCW.
#[inline]
fn cross(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Whether `p` lies inside (or on) triangle `abc`.
pub fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = cross(p, a, b);
    let d2 = cross(p, b, c);
    let d3 = cross(p, c, a);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

/// Ear-clip a single polygon loop (indices into `points`) into triangles.
/// The loop is reordered to CCW if needed. Returns triangles as index triples.
pub fn ear_clip_loop(points: &[Vec2], mut loop_idx: Vec<usize>) -> Vec<[usize; 3]> {
    if loop_idx.len() < 3 {
        return Vec::new();
    }
    if signed_area(points, &loop_idx) < 0.0 {
        loop_idx.reverse();
    }

    let mut tris = Vec::new();
    let mut guard = 0;
    while loop_idx.len() > 3 {
        let n = loop_idx.len();
        let mut clipped = false;
        for i in 0..n {
            let ip = loop_idx[(i + n - 1) % n];
            let ic = loop_idx[i];
            let inx = loop_idx[(i + 1) % n];
            let (a, b, c) = (points[ip], points[ic], points[inx]);
            // Convex corner (CCW) ...
            if cross(a, b, c) <= 0.0 {
                continue;
            }
            // ... with no other vertex inside the candidate ear.
            let mut empty = true;
            for &j in &loop_idx {
                if j == ip || j == ic || j == inx {
                    continue;
                }
                if point_in_triangle(points[j], a, b, c) {
                    empty = false;
                    break;
                }
            }
            if empty {
                tris.push([ip, ic, inx]);
                loop_idx.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            // Numerical fallback: clip an arbitrary corner to make progress.
            let n = loop_idx.len();
            tris.push([loop_idx[n - 1], loop_idx[0], loop_idx[1]]);
            loop_idx.remove(0);
        }
        guard += 1;
        if guard > 1_000_000 {
            break;
        }
    }
    if loop_idx.len() == 3 {
        tris.push([loop_idx[0], loop_idx[1], loop_idx[2]]);
    }
    tris
}

/// Triangulate a simple polygon (no holes). Triangles index into `polygon`.
pub fn triangulate(polygon: &[Vec2]) -> Vec<[usize; 3]> {
    ear_clip_loop(polygon, (0..polygon.len()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_area(points: &[Vec2], t: &[usize; 3]) -> f32 {
        cross(points[t[0]], points[t[1]], points[t[2]]).abs() * 0.5
    }

    #[test]
    fn square_makes_two_triangles() {
        let sq = vec![
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let tris = triangulate(&sq);
        assert_eq!(tris.len(), 2);
        let area: f32 = tris.iter().map(|t| tri_area(&sq, t)).sum();
        assert!((area - 1.0).abs() < 1e-5);
    }

    #[test]
    fn concave_polygon_triangulates_fully() {
        // An arrow / concave "C"-ish shape: 6 verts -> 4 triangles.
        let poly = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(1.0, 1.0), // concave notch
            Vec2::new(0.0, 2.0),
            Vec2::new(0.0, 1.0),
        ];
        let tris = triangulate(&poly);
        assert_eq!(tris.len(), poly.len() - 2);
        let total: f32 = tris.iter().map(|t| tri_area(&poly, t)).sum();
        // Area via shoelace.
        let shoelace = signed_area(&poly, &(0..poly.len()).collect::<Vec<_>>()).abs();
        assert!((total - shoelace).abs() < 1e-4, "{total} vs {shoelace}");
    }

    #[test]
    fn handles_clockwise_input() {
        let cw = vec![
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::ZERO,
        ];
        assert_eq!(triangulate(&cw).len(), 2);
    }
}
