//! A Lawson edge-flip pass that improves an existing triangulation toward the
//! Delaunay criterion (maximising the minimum angle, avoiding slivers).
//!
//! Only interior diagonals — edges shared by exactly two triangles — are
//! considered, so outer-boundary and hole-boundary edges (which border one
//! triangle) are never crossed. This makes it a *constrained* Delaunay improver:
//! it keeps the same triangulated region, just nicer triangles.

use std::collections::HashMap;

use glam::Vec2;

use crate::holes::{triangulate_with_holes, Triangulation};

#[inline]
fn cross(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// `> 0` when `d` lies inside the circumcircle of CCW triangle `abc`.
fn in_circle(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> f32 {
    let ax = a.x - d.x;
    let ay = a.y - d.y;
    let bx = b.x - d.x;
    let by = b.y - d.y;
    let cx = c.x - d.x;
    let cy = c.y - d.y;
    (ax * ax + ay * ay) * (bx * cy - cx * by)
        - (bx * bx + by * by) * (ax * cy - cx * ay)
        + (cx * cx + cy * cy) * (ax * by - bx * ay)
}

/// Order a triangle's indices CCW with respect to `points`.
fn make_ccw(points: &[Vec2], t: [usize; 3]) -> [usize; 3] {
    if cross(points[t[0]], points[t[1]], points[t[2]]) < 0.0 {
        [t[0], t[2], t[1]]
    } else {
        t
    }
}

#[inline]
fn edge_key(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

/// Apply Delaunay-improving flips in place. Returns the number of flips made.
pub fn delaunay_flip(tri: &mut Triangulation) -> usize {
    let pts = &tri.vertices;
    for t in tri.triangles.iter_mut() {
        *t = make_ccw(pts, *t);
    }

    let mut total = 0;
    let mut guard = 0;
    loop {
        // Map each edge to the triangles (and their apex vertex) touching it.
        let mut edges: HashMap<(usize, usize), Vec<(usize, usize)>> = HashMap::new();
        for (ti, t) in tri.triangles.iter().enumerate() {
            for k in 0..3 {
                let u = t[k];
                let v = t[(k + 1) % 3];
                let apex = t[(k + 2) % 3];
                edges.entry(edge_key(u, v)).or_default().push((ti, apex));
            }
        }

        let mut flipped = false;
        for (&(u, v), tris) in &edges {
            if tris.len() != 2 {
                continue; // boundary or non-manifold edge
            }
            let (t1, w) = tris[0];
            let (t2, x) = tris[1];
            let (pu, pv, pw, px) = (
                tri.vertices[u],
                tri.vertices[v],
                tri.vertices[w],
                tri.vertices[x],
            );
            // Convex quad? w and x must straddle edge u-v (they do), and u, v
            // must straddle edge w-x.
            let su = cross(pw, px, pu);
            let sv = cross(pw, px, pv);
            if su * sv >= 0.0 {
                continue; // non-convex: flipping would overlap
            }
            // Delaunay violated if x is inside circumcircle of CCW (u, v, w).
            let ccw = make_ccw(&tri.vertices, [u, v, w]);
            let inside = in_circle(
                tri.vertices[ccw[0]],
                tri.vertices[ccw[1]],
                tri.vertices[ccw[2]],
                px,
            );
            if inside > 1e-9 {
                tri.triangles[t1] = make_ccw(&tri.vertices, [w, x, u]);
                tri.triangles[t2] = make_ccw(&tri.vertices, [w, x, v]);
                flipped = true;
                total += 1;
                break; // edge map is now stale; rebuild
            }
        }

        guard += 1;
        if !flipped || guard > 100_000 {
            break;
        }
    }
    total
}

/// Triangulate an outer polygon with holes, then improve it with Delaunay flips.
pub fn triangulate_delaunay(outer: &[Vec2], holes: &[Vec<Vec2>]) -> Triangulation {
    let mut tri = triangulate_with_holes(outer, holes);
    delaunay_flip(&mut tri);
    tri
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(points: &[Vec2], t: &[usize; 3]) -> f32 {
        cross(points[t[0]], points[t[1]], points[t[2]]).abs() * 0.5
    }

    fn total_area(tri: &Triangulation) -> f32 {
        tri.triangles.iter().map(|t| area(&tri.vertices, t)).sum()
    }

    #[test]
    fn flip_preserves_area_and_count() {
        // A thin quad whose ear-clip diagonal is the long one; Delaunay prefers
        // the short diagonal but the count and area stay the same.
        let poly = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(11.0, 1.0),
            Vec2::new(1.0, 1.0),
        ];
        let mut tri = triangulate_delaunay(&poly, &[]);
        assert_eq!(tri.triangles.len(), 2);
        let a = total_area(&tri);
        // Polygon area via shoelace ~ 10.
        assert!((a - 10.0).abs() < 1e-3, "area {a}");

        // Idempotent: a second pass makes no further flips.
        assert_eq!(delaunay_flip(&mut tri), 0);
    }

    #[test]
    fn square_with_hole_area_preserved() {
        let outer = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 4.0),
            Vec2::new(0.0, 4.0),
        ];
        let hole = vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 3.0),
            Vec2::new(3.0, 3.0),
            Vec2::new(3.0, 1.0),
        ];
        let tri = triangulate_delaunay(&outer, &[hole]);
        assert!((total_area(&tri) - 12.0).abs() < 1e-3);
    }
}
