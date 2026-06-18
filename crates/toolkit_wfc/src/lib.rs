//! Tiled wave-function collapse.
//!
//! Describe a tile set and adjacency rules with a [`WfcModel`], then [`solve`]
//! a grid: the solver repeatedly collapses the lowest-entropy cell to a single
//! weighted tile and propagates the constraints. Deterministic for a given RNG
//! seed; returns `None` on an unsatisfiable contradiction.
//!
//! ```
//! use toolkit_wfc::{WfcModel, Dir, solve};
//! use toolkit_rng::Rng;
//!
//! let mut model = WfcModel::new();
//! let grass = model.add_tile(1.0);
//! let water = model.add_tile(1.0);
//! // Anything may border anything here.
//! for a in [grass, water] {
//!     for b in [grass, water] {
//!         model.allow(a, Dir::Right, b);
//!         model.allow(a, Dir::Up, b);
//!     }
//! }
//!
//! let grid = solve(&model, 8, 8, &mut Rng::seed_from_u64(42)).unwrap();
//! assert_eq!(grid.tiles.len(), 64);
//! ```

pub mod model;
pub mod solver;

pub use model::{Dir, WfcModel};
pub use solver::{solve, WfcGrid};
