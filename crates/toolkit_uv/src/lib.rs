//! UV unwrapping and atlas packing.
//!
//! The pipeline mirrors a real DCC tool:
//!
//! 1. **Mark seams** — choose edges to cut (see [`toolkit_topology::MeshSelection`]).
//! 2. **Segment** into charts ([`segment_charts`]) — connected patches between seams.
//! 3. **Unwrap** each chart with [LSCM](lscm) (angle-preserving) or a fast
//!    [projection](projection).
//! 4. **Pack** the charts into one texture space ([`pack_charts`]).
//!
//! ```
//! use glam::Vec3;
//! use toolkit_uv::{unwrap_charts, pack_charts};
//!
//! // A 2x2 quad grid (8 triangles) with no seams -> one chart.
//! let positions = vec![
//!     Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0),
//!     Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(2.0, 1.0, 0.0),
//! ];
//! let tris = vec![[0,3,1],[1,3,4],[1,4,2],[2,4,5]];
//! let mut charts = unwrap_charts(&positions, &tris, &[]);
//! pack_charts(&mut charts, 0.01);
//! assert_eq!(charts.len(), 1);
//! ```

pub mod atlas;
pub mod chart;
pub mod lscm;
pub mod projection;
pub mod solver;

pub use atlas::{pack_charts, pack_sizes, AtlasPlacement};
pub use chart::{segment_charts, unwrap_charts, Chart};
pub use lscm::{conformal_distortion, normalize_to_unit_square, unwrap_lscm, UnwrapResult};
pub use projection::{project_box, project_cylindrical, project_planar, project_spherical, Axis};
pub use solver::{solve_least_squares, SparseMatrix};
