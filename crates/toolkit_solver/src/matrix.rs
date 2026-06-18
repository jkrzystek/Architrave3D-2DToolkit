//! A sparse matrix in coordinate (triplet) form, plus the matrix-vector
//! products the iterative solvers need.
//!
//! Triplet form (`A[row, col] = val`) is the natural way assembly code emits a
//! system — each finite-element or constraint contribution is one `push`, and
//! duplicates at the same `(row, col)` sum (as they should in FEM assembly).

use serde::{Deserialize, Serialize};

/// A sparse matrix stored as accumulated `(row, col, value)` triplets.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SparseMatrix {
    pub rows: usize,
    pub cols: usize,
    triplets: Vec<(usize, usize, f32)>,
}

impl SparseMatrix {
    /// An empty `rows x cols` matrix.
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            triplets: Vec::new(),
        }
    }

    /// Accumulate `val` at `(row, col)`. Zeros are dropped; duplicate entries at
    /// the same position sum.
    pub fn push(&mut self, row: usize, col: usize, val: f32) {
        debug_assert!(row < self.rows && col < self.cols, "index out of range");
        if val != 0.0 {
            self.triplets.push((row, col, val));
        }
    }

    /// Number of stored (non-zero) entries, before duplicate consolidation.
    pub fn nnz(&self) -> usize {
        self.triplets.len()
    }

    /// `y = A x`
    pub fn mul(&self, x: &[f32]) -> Vec<f32> {
        assert_eq!(x.len(), self.cols, "vector length must equal column count");
        let mut y = vec![0.0; self.rows];
        for &(r, c, v) in &self.triplets {
            y[r] += v * x[c];
        }
        y
    }

    /// `z = Aᵀ y`
    pub fn mul_transpose(&self, y: &[f32]) -> Vec<f32> {
        assert_eq!(y.len(), self.rows, "vector length must equal row count");
        let mut z = vec![0.0; self.cols];
        for &(r, c, v) in &self.triplets {
            z[c] += v * y[r];
        }
        z
    }

    /// Consolidate triplets into per-row `(col, value)` lists with duplicates
    /// summed. Used by the stationary iterative solvers, which need row access.
    pub fn compress_rows(&self) -> Vec<Vec<(usize, f32)>> {
        let mut rows: Vec<Vec<(usize, f32)>> = vec![Vec::new(); self.rows];
        for &(r, c, v) in &self.triplets {
            let row = &mut rows[r];
            if let Some(entry) = row.iter_mut().find(|(col, _)| *col == c) {
                entry.1 += v;
            } else {
                row.push((c, v));
            }
        }
        rows
    }

    /// The diagonal entries `A[i, i]` (zero where absent).
    pub fn diagonal(&self) -> Vec<f32> {
        let n = self.rows.min(self.cols);
        let mut d = vec![0.0; n];
        for &(r, c, v) in &self.triplets {
            if r == c && r < n {
                d[r] += v;
            }
        }
        d
    }
}

/// Dot product of two equal-length slices.
pub(crate) fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matvec_and_transpose() {
        let mut a = SparseMatrix::new(2, 3);
        a.push(0, 0, 1.0);
        a.push(0, 1, 2.0);
        a.push(1, 2, 3.0);
        assert_eq!(a.mul(&[1.0, 1.0, 1.0]), vec![3.0, 3.0]);
        assert_eq!(a.mul_transpose(&[1.0, 1.0]), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn duplicates_sum() {
        let mut a = SparseMatrix::new(1, 1);
        a.push(0, 0, 1.0);
        a.push(0, 0, 2.0);
        assert_eq!(a.mul(&[1.0]), vec![3.0]);
        assert_eq!(a.diagonal(), vec![3.0]);
    }

    #[test]
    fn compress_rows_consolidates() {
        let mut a = SparseMatrix::new(2, 2);
        a.push(0, 0, 1.0);
        a.push(0, 0, 1.0);
        a.push(0, 1, 2.0);
        let rows = a.compress_rows();
        assert!(rows[0].contains(&(0, 2.0)));
        assert!(rows[0].contains(&(1, 2.0)));
        assert!(rows[1].is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let mut a = SparseMatrix::new(2, 2);
        a.push(0, 0, 5.0);
        let json = serde_json::to_string(&a).unwrap();
        let back: SparseMatrix = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }
}
