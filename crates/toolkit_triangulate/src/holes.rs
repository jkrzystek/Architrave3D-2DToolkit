//! Triangulating a polygon with holes, by bridging each hole into the outer
//! loop and then ear-clipping the resulting simple polygon.
//!
//! Holes are linked by a "bridge" edge from the hole's rightmost vertex to a
//! visible vertex on the current outer loop (the earcut approach), which turns a
//! ring-with-holes into a single, self-touching simple polygon.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::earclip::{ear_clip_loop, signed_area};

/// A triangulation result: the combined vertex list (outer points followed by
/// each hole's points) and triangles indexing into it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Triangulation {
    pub vertices: Vec<Vec2>,
    pub triangles: Vec<[usize; 3]>,
}

/// Index of the maximum-x vertex within `range` of `points`.
fn rightmost(points: &[Vec2], range: std::ops::Range<usize>) -> usize {
    let mut best = range.start;
    for i in range {
        if points[i].x > points[best].x {
            best = i;
        }
    }
    best
}

/// Find a vertex on the current `loop_idx` to bridge the hole vertex `m` to:
/// the right-intersected outer edge's endpoint with the greater x.
fn find_bridge(points: &[Vec2], loop_idx: &[usize], m_global: usize) -> Option<usize> {
    let m = points[m_global];
    let n = loop_idx.len();
    let mut best_x = f32::INFINITY;
    let mut bridge = None;
    for i in 0..n {
        let ai = loop_idx[i];
        let bi = loop_idx[(i + 1) % n];
        let a = points[ai];
        let b = points[bi];
        // Edge straddles the horizontal line y = m.y.
        let straddles = (a.y <= m.y && b.y >= m.y) || (b.y <= m.y && a.y >= m.y);
        if !straddles || (a.y - b.y).abs() < 1e-20 {
            continue;
        }
        let t = (m.y - a.y) / (b.y - a.y);
        let x = a.x + t * (b.x - a.x);
        if x >= m.x - 1e-9 && x < best_x {
            best_x = x;
            bridge = Some(if a.x >= b.x { ai } else { bi });
        }
    }
    bridge
}

/// Triangulate an outer polygon with zero or more holes.
///
/// `outer` should wind counter-clockwise and `holes` clockwise; either way the
/// code reorients them (outer CCW, holes CW) before bridging.
pub fn triangulate_with_holes(outer: &[Vec2], holes: &[Vec<Vec2>]) -> Triangulation {
    // Combined vertex buffer: outer first, then each hole.
    let mut vertices: Vec<Vec2> = outer.to_vec();
    // Outer loop, forced CCW.
    let mut loop_idx: Vec<usize> = (0..outer.len()).collect();
    if signed_area(&vertices, &loop_idx) < 0.0 {
        loop_idx.reverse();
    }

    // Prepare holes with their global index ranges, sorted by descending max-x
    // so outer-most holes bridge first.
    struct HoleRef {
        start: usize,
        len: usize,
        max_x: f32,
    }
    let mut hole_refs: Vec<HoleRef> = Vec::new();
    for hole in holes {
        if hole.len() < 3 {
            continue;
        }
        let start = vertices.len();
        // Force CW winding for holes (negative signed area).
        let mut h = hole.clone();
        let area: f32 = {
            let idx: Vec<usize> = (0..h.len()).collect();
            signed_area(&h, &idx)
        };
        if area > 0.0 {
            h.reverse();
        }
        let max_x = h.iter().fold(f32::NEG_INFINITY, |m, p| m.max(p.x));
        vertices.extend_from_slice(&h);
        hole_refs.push(HoleRef {
            start,
            len: h.len(),
            max_x,
        });
    }
    hole_refs.sort_by(|a, b| b.max_x.partial_cmp(&a.max_x).unwrap_or(std::cmp::Ordering::Equal));

    // Bridge each hole into the loop.
    for hr in &hole_refs {
        let m = rightmost(&vertices, hr.start..hr.start + hr.len);
        let outer_v = match find_bridge(&vertices, &loop_idx, m) {
            Some(v) => v,
            None => continue,
        };
        // Build the hole's index ring rotated to start at `m`, CW order as stored.
        let mut hole_ring: Vec<usize> = (hr.start..hr.start + hr.len).collect();
        let m_pos = hole_ring.iter().position(|&v| v == m).unwrap();
        hole_ring.rotate_left(m_pos);

        // Splice: ... outer_v, [hole ring from m around back to m], outer_v, ...
        let pos = loop_idx.iter().position(|&v| v == outer_v).unwrap();
        let mut spliced = Vec::with_capacity(loop_idx.len() + hr.len + 2);
        spliced.extend_from_slice(&loop_idx[..=pos]);
        spliced.extend_from_slice(&hole_ring); // m .. m_prev
        spliced.push(m); // close the hole back to its start
        spliced.push(outer_v); // bridge back to the outer loop
        spliced.extend_from_slice(&loop_idx[pos + 1..]);
        loop_idx = spliced;
    }

    let triangles = ear_clip_loop(&vertices, loop_idx);
    Triangulation {
        vertices,
        triangles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(points: &[Vec2], t: &[usize; 3]) -> f32 {
        let a = points[t[0]];
        let b = points[t[1]];
        let c = points[t[2]];
        ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() * 0.5
    }

    #[test]
    fn square_with_square_hole() {
        // Outer 4x4 square, centered 2x2 hole -> area 16 - 4 = 12.
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
        let tri = triangulate_with_holes(&outer, &[hole]);
        let total: f32 = tri.triangles.iter().map(|t| area(&tri.vertices, t)).sum();
        assert!((total - 12.0).abs() < 1e-3, "covered area {total}, expected 12");
        assert!(!tri.triangles.is_empty());
    }

    #[test]
    fn no_holes_matches_simple_triangulation() {
        let outer = vec![
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let tri = triangulate_with_holes(&outer, &[]);
        assert_eq!(tri.triangles.len(), 2);
    }

    #[test]
    fn serde_roundtrip() {
        let outer = vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)];
        let tri = triangulate_with_holes(&outer, &[]);
        let json = serde_json::to_string(&tri).unwrap();
        let back: Triangulation = serde_json::from_str(&json).unwrap();
        assert_eq!(tri, back);
    }
}
