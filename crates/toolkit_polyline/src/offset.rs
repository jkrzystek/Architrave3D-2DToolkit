//! 2D polyline offsetting (the pen/stroke "outline" operation).

use glam::Vec2;

/// Offset a 2D polyline by `distance` along its per-vertex normals (positive is
/// to the right of travel). Each vertex normal averages its incident edge
/// normals, so corners stay watertight for gentle angles. `closed` wraps the
/// ends. Sharp corners are not mitred (kept simple); resample first if needed.
pub fn offset_2d(points: &[Vec2], distance: f32, closed: bool) -> Vec<Vec2> {
    let n = points.len();
    if n < 2 {
        return points.to_vec();
    }

    // Right-hand normal of an edge a->b.
    let edge_normal = |a: Vec2, b: Vec2| -> Vec2 {
        let d = b - a;
        let len = d.length();
        if len <= 1e-12 {
            Vec2::ZERO
        } else {
            Vec2::new(d.y, -d.x) / len
        }
    };

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // Normals of the edges entering and leaving vertex i.
        let incoming = if i > 0 || closed {
            let prev = points[(i + n - 1) % n];
            Some(edge_normal(prev, points[i]))
        } else {
            None
        };
        let outgoing = if i + 1 < n || closed {
            let next = points[(i + 1) % n];
            Some(edge_normal(points[i], next))
        } else {
            None
        };

        let normal = match (incoming, outgoing) {
            (Some(a), Some(b)) => {
                let sum = a + b;
                if sum.length() > 1e-9 {
                    sum.normalize()
                } else {
                    b
                }
            }
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => Vec2::ZERO,
        };
        out.push(points[i] + normal * distance);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_line_offsets_sideways() {
        // Horizontal line going +x; right-hand normal points -y.
        let pts = vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)];
        let off = offset_2d(&pts, 1.0, false);
        for p in &off {
            assert!((p.y - (-1.0)).abs() < 1e-5, "y should be -1, got {}", p.y);
        }
    }

    #[test]
    fn negative_distance_flips_side() {
        let pts = vec![Vec2::ZERO, Vec2::new(1.0, 0.0)];
        let off = offset_2d(&pts, -1.0, false);
        assert!((off[0].y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn short_input_passthrough() {
        let pts = vec![Vec2::ONE];
        assert_eq!(offset_2d(&pts, 1.0, false), pts);
    }
}
