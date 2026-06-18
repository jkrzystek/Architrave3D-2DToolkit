//! Sparse matrices and iterative linear solvers, dependency-free.
//!
//! Assemble a system as a [`SparseMatrix`] in triplet form, then solve it:
//! [`solve_least_squares`] (CGLS) for any least-squares problem,
//! [`solve_cg`] for symmetric positive-definite systems (Laplacian smoothing,
//! deformation, pressure projection), or [`solve_jacobi`] /
//! [`solve_gauss_seidel`] for relaxation. This generalises the solver that
//! previously lived inside `toolkit_uv` so fluids, deformers, and unwrapping can
//! all share it.
//!
//! ```
//! use toolkit_solver::{SparseMatrix, solve_cg};
//!
//! // [[4,1],[1,3]] x = [1,2]  ->  x ≈ (0.0909, 0.6364)
//! let mut a = SparseMatrix::new(2, 2);
//! a.push(0, 0, 4.0); a.push(0, 1, 1.0);
//! a.push(1, 0, 1.0); a.push(1, 1, 3.0);
//! let x = solve_cg(&a, &[1.0, 2.0], 50, 1e-10);
//! assert!((x[0] - 1.0 / 11.0).abs() < 1e-3);
//! ```

pub mod iterative;
pub mod matrix;

pub use iterative::{solve_cg, solve_gauss_seidel, solve_jacobi, solve_least_squares};
pub use matrix::SparseMatrix;
