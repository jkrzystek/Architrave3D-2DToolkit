//! A minimal sparse linear least-squares solver (CGLS / conjugate gradient on
//! the normal equations) used by LSCM unwrapping. Kept dependency-free so the
//! toolkit does not pull in a linear-algebra crate for one algorithm.

/// A sparse matrix in coordinate (triplet) form: `A[row, col] = val`.
#[derive(Clone, Debug, Default)]
pub struct SparseMatrix {
    pub rows: usize,
    pub cols: usize,
    triplets: Vec<(usize, usize, f32)>,
}

impl SparseMatrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            triplets: Vec::new(),
        }
    }

    /// Accumulate a value at `(row, col)` (duplicate entries sum, as in FEM).
    pub fn push(&mut self, row: usize, col: usize, val: f32) {
        if val != 0.0 {
            self.triplets.push((row, col, val));
        }
    }

    /// `y = A x`
    pub fn mul(&self, x: &[f32]) -> Vec<f32> {
        let mut y = vec![0.0; self.rows];
        for &(r, c, v) in &self.triplets {
            y[r] += v * x[c];
        }
        y
    }

    /// `z = Aᵀ y`
    pub fn mul_transpose(&self, y: &[f32]) -> Vec<f32> {
        let mut z = vec![0.0; self.cols];
        for &(r, c, v) in &self.triplets {
            z[c] += v * y[r];
        }
        z
    }
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Solve `min_x ||A x - b||²` with the conjugate-gradient least-squares (CGLS)
/// method. Returns the solution vector of length `A.cols`.
pub fn solve_least_squares(a: &SparseMatrix, b: &[f32], max_iters: usize, tol: f32) -> Vec<f32> {
    let n = a.cols;
    let mut x = vec![0.0; n];

    // r = b - A x = b (x starts at zero)
    let mut r = b.to_vec();
    // s = Aᵀ r
    let mut s = a.mul_transpose(&r);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_identity_system() {
        // A = I (3x3), b = [1,2,3] -> x = b
        let mut a = SparseMatrix::new(3, 3);
        a.push(0, 0, 1.0);
        a.push(1, 1, 1.0);
        a.push(2, 2, 1.0);
        let x = solve_least_squares(&a, &[1.0, 2.0, 3.0], 50, 1e-8);
        assert!((x[0] - 1.0).abs() < 1e-4);
        assert!((x[1] - 2.0).abs() < 1e-4);
        assert!((x[2] - 3.0).abs() < 1e-4);
    }

    #[test]
    fn solves_overdetermined_least_squares() {
        // Fit y = m x through (0,0),(1,1),(2,2.2): rows = [0;1;2], b=[0;1;2.2]
        // Least squares slope m = (sum xy)/(sum x^2) = (0+1+4.4)/(0+1+4)=5.4/5=1.08
        let mut a = SparseMatrix::new(3, 1);
        a.push(0, 0, 0.0);
        a.push(1, 0, 1.0);
        a.push(2, 0, 2.0);
        let x = solve_least_squares(&a, &[0.0, 1.0, 2.2], 100, 1e-10);
        assert!((x[0] - 1.08).abs() < 1e-3, "got {}", x[0]);
    }

    #[test]
    fn transpose_matvec() {
        let mut a = SparseMatrix::new(2, 3);
        a.push(0, 0, 1.0);
        a.push(0, 1, 2.0);
        a.push(1, 2, 3.0);
        // Aᵀ [1,1] = [1,2,3]
        let z = a.mul_transpose(&[1.0, 1.0]);
        assert_eq!(z, vec![1.0, 2.0, 3.0]);
    }
}
