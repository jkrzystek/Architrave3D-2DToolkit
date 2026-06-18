//! Weighted (soft) selection sets, element-kind agnostic.
//!
//! A [`Selection`] maps element indices to weights in `[0, 1]`. Hard selections
//! (all weights `1`) and soft selections (falloff weights for proportional
//! editing) use the same type, so move/scale/paint tools just multiply by the
//! weight. It complements `toolkit_topology::MeshSelection` (which is hard and
//! mode-bound) by adding weights, boolean ops, grow/shrink over an
//! [`Adjacency`], attribute thresholding, and distance falloff.
//!
//! ```
//! use toolkit_select::{Selection, Adjacency};
//!
//! // Select a vertex, then grow one ring across an edge graph.
//! let adj = Adjacency::from_pairs(4, &[(0, 1), (1, 2), (2, 3)]);
//! let grown = Selection::from_indices([1]).grow(&adj);
//! assert!(grown.contains(0) && grown.contains(2));
//! ```

pub mod adjacency;
pub mod selection;

pub use adjacency::Adjacency;
pub use selection::Selection;
