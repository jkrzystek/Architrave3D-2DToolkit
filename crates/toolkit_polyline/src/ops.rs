//! Dimension-independent polyline operations (work for `Vec2` and `Vec3`).

use crate::point::Point;

/// Total arc length of a polyline.
pub fn length<P: Point>(points: &[P]) -> f32 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Resample a polyline into exactly `count` points spread evenly by arc length,
/// including both endpoints. `count < 2` or a degenerate path returns a copy.
pub fn resample<P: Point>(points: &[P], count: usize) -> Vec<P> {
    if points.len() < 2 || count < 2 {
        return points.to_vec();
    }
    let total = length(points);
    if total <= 1e-12 {
        return vec![points[0]; count];
    }
    let step = total / (count - 1) as f32;
    let mut out = Vec::with_capacity(count);
    out.push(points[0]);

    let mut seg = 0usize;
    let mut walked = 0.0f32; // arc length at the start of segment `seg`
    for k in 1..count - 1 {
        let target = step * k as f32;
        while seg < points.len() - 1 {
            let seg_len = points[seg].distance(points[seg + 1]);
            if walked + seg_len >= target || seg == points.len() - 2 {
                let t = if seg_len > 1e-12 {
                    ((target - walked) / seg_len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                out.push(points[seg].lerp(points[seg + 1], t));
                break;
            }
            walked += seg_len;
            seg += 1;
        }
    }
    out.push(*points.last().unwrap());
    out
}

/// Resample so consecutive points are `spacing` apart (the last gap may be
/// shorter). The endpoints are preserved.
pub fn resample_by_spacing<P: Point>(points: &[P], spacing: f32) -> Vec<P> {
    let total = length(points);
    if points.len() < 2 || spacing <= 1e-12 || total <= 1e-12 {
        return points.to_vec();
    }
    let count = (total / spacing).floor() as usize + 1;
    resample(points, count.max(2))
}

/// Chaikin corner-cutting subdivision: each iteration replaces every edge with
/// two points at 1/4 and 3/4, smoothing the curve. Endpoints are kept for open
/// polylines; `closed` wraps around.
pub fn smooth_chaikin<P: Point>(points: &[P], iterations: usize, closed: bool) -> Vec<P> {
    let mut cur = points.to_vec();
    for _ in 0..iterations {
        let n = cur.len();
        if n < 2 {
            break;
        }
        let mut next = Vec::with_capacity(n * 2);
        if !closed {
            next.push(cur[0]);
        }
        let edges = if closed { n } else { n - 1 };
        for i in 0..edges {
            let a = cur[i];
            let b = cur[(i + 1) % n];
            next.push(a.lerp(b, 0.25));
            next.push(a.lerp(b, 0.75));
        }
        if !closed {
            next.push(cur[n - 1]);
        }
        cur = next;
    }
    cur
}

/// Laplacian smoothing: each interior point moves a `factor` of the way toward
/// the average of its neighbours. Open endpoints stay fixed; `closed` wraps.
pub fn smooth_laplacian<P: Point>(
    points: &[P],
    iterations: usize,
    factor: f32,
    closed: bool,
) -> Vec<P> {
    let mut cur = points.to_vec();
    let n = cur.len();
    if n < 3 {
        return cur;
    }
    for _ in 0..iterations {
        let mut next = cur.clone();
        let (lo, hi) = if closed { (0, n) } else { (1, n - 1) };
        for i in lo..hi {
            let prev = cur[(i + n - 1) % n];
            let nxt = cur[(i + 1) % n];
            let avg = prev.add(nxt).scale(0.5);
            next[i] = cur[i].lerp(avg, factor);
        }
        cur = next;
    }
    cur
}

/// Distance from `p` to the infinite line through `a` and `b`.
fn line_distance<P: Point>(p: P, a: P, b: P) -> f32 {
    let ab = b.sub(a);
    let denom = ab.dot(ab);
    let ap = p.sub(a);
    if denom <= 1e-12 {
        return ap.length();
    }
    let proj = ab.scale(ap.dot(ab) / denom);
    ap.sub(proj).length()
}

/// Douglas-Peucker simplification: drop points that lie within `tolerance` of
/// the line between retained neighbours. Endpoints are always kept.
pub fn simplify<P: Point>(points: &[P], tolerance: f32) -> Vec<P> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;

    // Iterative DP over a stack of (first, last) ranges.
    let mut stack = vec![(0usize, n - 1)];
    while let Some((first, last)) = stack.pop() {
        let mut max_d = 0.0;
        let mut idx = first;
        for i in (first + 1)..last {
            let d = line_distance(points[i], points[first], points[last]);
            if d > max_d {
                max_d = d;
                idx = i;
            }
        }
        if max_d > tolerance && idx != first {
            keep[idx] = true;
            stack.push((first, idx));
            stack.push((idx, last));
        }
    }

    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| if k { Some(*p) } else { None })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec2, Vec3};

    #[test]
    fn length_of_unit_steps() {
        let pts = vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0)];
        assert!((length(&pts) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn resample_evenly_spaces() {
        let pts = vec![Vec2::ZERO, Vec2::new(4.0, 0.0)];
        let r = resample(&pts, 5);
        assert_eq!(r.len(), 5);
        for (i, p) in r.iter().enumerate() {
            assert!((p.x - i as f32).abs() < 1e-5, "point {i} at {p:?}");
        }
    }

    #[test]
    fn resample_by_spacing_count() {
        let pts = vec![Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)];
        let r = resample_by_spacing(&pts, 2.5);
        // 10 / 2.5 = 4 -> 5 points.
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn chaikin_increases_points_and_keeps_endpoints() {
        let pts = vec![Vec2::ZERO, Vec2::new(1.0, 1.0), Vec2::new(2.0, 0.0)];
        let s = smooth_chaikin(&pts, 1, false);
        assert_eq!(s.first().copied(), Some(Vec2::ZERO));
        assert_eq!(s.last().copied(), Some(Vec2::new(2.0, 0.0)));
        assert!(s.len() > pts.len());
    }

    #[test]
    fn laplacian_relaxes_spike() {
        // A spike at the middle should be pulled toward the line.
        let pts = vec![Vec2::ZERO, Vec2::new(1.0, 5.0), Vec2::new(2.0, 0.0)];
        let s = smooth_laplacian(&pts, 1, 1.0, false);
        assert!(s[1].y < 5.0);
        // Endpoints unchanged.
        assert_eq!(s[0], Vec2::ZERO);
        assert_eq!(s[2], Vec2::new(2.0, 0.0));
    }

    #[test]
    fn simplify_drops_collinear_points() {
        let pts = vec![
            Vec2::ZERO,
            Vec2::new(1.0, 0.001),
            Vec2::new(2.0, 0.0),
            Vec2::new(3.0, 2.0),
        ];
        let s = simplify(&pts, 0.01);
        // The nearly-collinear middle point is removed; the corner stays.
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], Vec2::ZERO);
        assert_eq!(s.last().copied(), Some(Vec2::new(3.0, 2.0)));
    }
}
