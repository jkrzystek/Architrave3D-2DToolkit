//! Knot-vector utilities shared by B-spline and NURBS evaluation.

use glam::Vec4;

/// De Boor's algorithm in homogeneous (4D) coordinates, used by NURBS curves
/// and surfaces. `n` is the last control index (`control.len() - 1`).
pub fn de_boor4(n: usize, degree: usize, u: f32, knots: &[f32], control: &[Vec4]) -> Vec4 {
    let span = find_span(n, degree, u, knots);
    let mut d: Vec<Vec4> = (0..=degree).map(|i| control[span - degree + i]).collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denom = knots[i + degree - r + 1] - knots[i];
            let alpha = if denom.abs() < 1e-9 {
                0.0
            } else {
                (u - knots[i]) / denom
            };
            d[j] = d[j - 1].lerp(d[j], alpha);
        }
    }
    d[degree]
}

/// Generate a clamped, uniform knot vector for `n_control` control points of
/// the given `degree`. The curve passes through its first and last control
/// points and is defined over `[0, 1]`.
///
/// Length is `n_control + degree + 1`.
pub fn clamped_uniform_knots(n_control: usize, degree: usize) -> Vec<f32> {
    let m = n_control + degree + 1;
    let mut knots = vec![0.0; m];
    let inner = n_control - degree; // number of distinct interior steps
    for (i, k) in knots.iter_mut().enumerate() {
        *k = if i <= degree {
            0.0
        } else if i >= n_control {
            1.0
        } else {
            (i - degree) as f32 / inner as f32
        };
    }
    knots
}

/// Find the knot span index `s` such that `knots[s] <= u < knots[s+1]`, using
/// the convention from *The NURBS Book* (Piegl & Tiller). `n` is the index of
/// the last control point (`n_control - 1`).
pub fn find_span(n: usize, degree: usize, u: f32, knots: &[f32]) -> usize {
    if u >= knots[n + 1] {
        return n;
    }
    if u <= knots[degree] {
        return degree;
    }
    let mut low = degree;
    let mut high = n + 1;
    let mut mid = (low + high) / 2;
    while u < knots[mid] || u >= knots[mid + 1] {
        if u < knots[mid] {
            high = mid;
        } else {
            low = mid;
        }
        mid = (low + high) / 2;
    }
    mid
}

/// The valid parameter domain `[min, max]` for a knot vector and degree.
pub fn domain(knots: &[f32], degree: usize) -> (f32, f32) {
    (knots[degree], knots[knots.len() - 1 - degree])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamped_knots_have_correct_length() {
        let k = clamped_uniform_knots(5, 3);
        assert_eq!(k.len(), 5 + 3 + 1);
        // Clamped: first/last degree+1 knots are 0 and 1.
        assert_eq!(&k[0..4], &[0.0, 0.0, 0.0, 0.0]);
        assert_eq!(&k[5..9], &[1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn span_lookup() {
        // degree 2, 4 control points -> knots [0,0,0,0.5,1,1,1]
        let k = clamped_uniform_knots(4, 2);
        let n = 3;
        assert_eq!(find_span(n, 2, 0.0, &k), 2);
        assert_eq!(find_span(n, 2, 1.0, &k), 3);
        let s = find_span(n, 2, 0.25, &k);
        assert!(k[s] <= 0.25 && 0.25 < k[s + 1]);
    }

    #[test]
    fn domain_is_unit() {
        let k = clamped_uniform_knots(6, 3);
        let (a, b) = domain(&k, 3);
        assert_eq!((a, b), (0.0, 1.0));
    }
}
