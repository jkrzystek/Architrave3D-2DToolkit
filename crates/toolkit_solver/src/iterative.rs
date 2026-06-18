//! Iterative solvers for sparse linear systems.
//!
//! - [`solve_least_squares`] (CGLS) minimises `‖A x − b‖²` for any (even
//!   rectangular) `A` — the workhorse for least-squares fits, LSCM unwrapping,
//!   and gradient-domain problems.
//! - [`solve_cg`] is conjugate gradient for a symmetric positive-definite `A`
//!   (Poisson / Laplacian systems: smoothing, deformation, pressure solves).
//! - [`solve_jacobi`] and [`solve_gauss_seidel`] are simple stationary
//!   iterations, handy for diagonally dominant systems and relaxation passes.

use crate::matrix::{dot, SparseMatrix};

/// Solve `min_x ‖A x − b‖²` via conjugate-gradient least squares (CGLS).
/// Returns a vector of length `A.cols`.
pub fn solve_least_squares(a: &SparseMatrix, b: &[f32], max_iters: usize, tol: f32) -> Vec<f32> {
    let n = a.cols;
    let mut x = vec![0.0; n];

    let mut r = b.to_vec(); // r = b - A x, x = 0
    let mut s = a.mul_transpose(&r); // s = Aᵀ r
    let mut p = s.clone();
    let mut gamma = dot(&s, &s);
    let initial = gamma.sqrt().max(1e-20);

    for _ in 0..max_iters {
        let q = a.mul(&p);
        let denom = dot(&q, &q);
        if denom <= 1e-30 {
            break;
        }
        let alpha = gamma / denom;
        for i in 0..n {
            x[i] += alpha * p[i];
        }
        for i in 0..r.len() {
            r[i] -= alpha * q[i];
        }
        s = a.mul_transpose(&r);
        let gamma_new = dot(&s, &s);
        if gamma_new.sqrt() / initial < tol {
            break;
        }
        let beta = gamma_new / gamma;
        for i in 0..n {
            p[i] = s[i] + beta * p[i];
        }
        gamma = gamma_new;
    }
    x
}

/// Solve `A x = b` for a symmetric positive-definite `A` with conjugate
/// gradient. `A` must be square; behaviour is undefined if it is not SPD.
pub fn solve_cg(a: &SparseMatrix, b: &[f32], max_iters: usize, tol: f32) -> Vec<f32> {
    assert_eq!(a.rows, a.cols, "CG requires a square matrix");
    let n = a.cols;
    let mut x = vec![0.0; n];

    let mut r = b.to_vec(); // r = b - A x, x = 0
    let mut p = r.clone();
    let mut rs_old = dot(&r, &r);
    let initial = rs_old.sqrt().max(1e-20);

    for _ in 0..max_iters {
        let ap = a.mul(&p);
        let denom = dot(&p, &ap);
        if denom.abs() <= 1e-30 {
            break;
        }
        let alpha = rs_old / denom;
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        let rs_new = dot(&r, &r);
        if rs_new.sqrt() / initial < tol {
            break;
        }
        let beta = rs_new / rs_old;
        for i in 0..n {
            p[i] = r[i] + beta * p[i];
        }
        rs_old = rs_new;
    }
    x
}

/// Solve `A x = b` with Jacobi iteration. Converges for diagonally dominant `A`.
/// Returns `x` after at most `max_iters` sweeps (early-out when the update
/// shrinks below `tol`).
pub fn solve_jacobi(a: &SparseMatrix, b: &[f32], max_iters: usize, tol: f32) -> Vec<f32> {
    assert_eq!(a.rows, a.cols, "Jacobi requires a square matrix");
    let n = a.cols;
    let rows = a.compress_rows();
    let diag = a.diagonal();
    let mut x = vec![0.0; n];
    let mut next = vec![0.0; n];

    for _ in 0..max_iters {
        for i in 0..n {
            let mut sigma = 0.0;
            for &(c, v) in &rows[i] {
                if c != i {
                    sigma += v * x[c];
                }
            }
            let d = diag[i];
            next[i] = if d.abs() > 1e-30 { (b[i] - sigma) / d } else { x[i] };
        }
        let delta: f32 = next.iter().zip(&x).map(|(a, b)| (a - b) * (a - b)).sum();
        x.copy_from_slice(&next);
        if delta.sqrt() < tol {
            break;
        }
    }
    x
}

/// Solve `A x = b` with Gauss-Seidel iteration (in-place updates, so it usually
/// converges faster than Jacobi for the same systems).
pub fn solve_gauss_seidel(a: &SparseMatrix, b: &[f32], max_iters: usize, tol: f32) -> Vec<f32> {
    assert_eq!(a.rows, a.cols, "Gauss-Seidel requires a square matrix");
    let n = a.cols;
    let rows = a.compress_rows();
    let diag = a.diagonal();
    let mut x = vec![0.0; n];

    for _ in 0..max_iters {
        let mut delta = 0.0;
        for i in 0..n {
            let mut sigma = 0.0;
            for &(c, v) in &rows[i] {
                if c != i {
                    sigma += v * x[c];
                }
            }
            let d = diag[i];
            if d.abs() > 1e-30 {
                let xi = (b[i] - sigma) / d;
                delta += (xi - x[i]) * (xi - x[i]);
                x[i] = xi;
            }
        }
        if delta.sqrt() < tol {
            break;
        }
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build the symmetric PD system from a small example: [[4,1],[1,3]] x = [1,2].
    fn spd_2x2() -> (SparseMatrix, Vec<f32>) {
        let mut a = SparseMatrix::new(2, 2);
        a.push(0, 0, 4.0);
        a.push(0, 1, 1.0);
        a.push(1, 0, 1.0);
        a.push(1, 1, 3.0);
        (a, vec![1.0, 2.0])
    }

    // Exact solution: x = (1/11, 7/11) ≈ (0.0909, 0.6364).
    fn check_2x2(x: &[f32]) {
        assert!((x[0] - 1.0 / 11.0).abs() < 1e-3, "x0 = {}", x[0]);
        assert!((x[1] - 7.0 / 11.0).abs() < 1e-3, "x1 = {}", x[1]);
    }

    #[test]
    fn cg_solves_spd() {
        let (a, b) = spd_2x2();
        check_2x2(&solve_cg(&a, &b, 50, 1e-10));
    }

    #[test]
    fn jacobi_solves_spd() {
        let (a, b) = spd_2x2();
        check_2x2(&solve_jacobi(&a, &b, 500, 1e-10));
    }

    #[test]
    fn gauss_seidel_solves_spd() {
        let (a, b) = spd_2x2();
        check_2x2(&solve_gauss_seidel(&a, &b, 200, 1e-10));
    }

    #[test]
    fn cgls_solves_overdetermined() {
        // Fit slope through (0,0),(1,1),(2,2.2): least-squares m = 1.08.
        let mut a = SparseMatrix::new(3, 1);
        a.push(1, 0, 1.0);
        a.push(2, 0, 2.0);
        let x = solve_least_squares(&a, &[0.0, 1.0, 2.2], 100, 1e-10);
        assert!((x[0] - 1.08).abs() < 1e-3, "got {}", x[0]);
    }

    #[test]
    fn cg_identity_returns_b() {
        let mut a = SparseMatrix::new(3, 3);
        a.push(0, 0, 1.0);
        a.push(1, 1, 1.0);
        a.push(2, 2, 1.0);
        let x = solve_cg(&a, &[3.0, -1.0, 2.0], 10, 1e-12);
        assert!((x[0] - 3.0).abs() < 1e-6 && (x[1] + 1.0).abs() < 1e-6 && (x[2] - 2.0).abs() < 1e-6);
    }
}
