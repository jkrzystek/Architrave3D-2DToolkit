//! Parametric curves and surfaces — the foundation for CAD-style modeling.
//!
//! * [`Bezier`] — arbitrary-degree Bézier curves (de Casteljau).
//! * [`BSplineCurve`] — B-splines with clamped/uniform or custom knots (De Boor).
//! * [`NurbsCurve`] / [`NurbsSurface`] — rational B-splines; weights let them
//!   represent circles and conics exactly.
//! * [`CatmullRom`] — interpolating spline through waypoints.
//!
//! Curves tessellate to polylines; surfaces tessellate to a
//! [`toolkit_geometry::Mesh`], so results drop straight into the renderer.
//!
//! ```
//! use glam::Vec3;
//! use toolkit_curves::BSplineCurve;
//!
//! let curve = BSplineCurve::new(
//!     vec![Vec3::ZERO, Vec3::new(1.0, 2.0, 0.0), Vec3::new(3.0, 2.0, 0.0), Vec3::new(4.0, 0.0, 0.0)],
//!     3,
//! );
//! let polyline = curve.tessellate(32);
//! assert_eq!(polyline.len(), 33);
//! ```

pub mod bezier;
pub mod bspline;
pub mod catmull_rom;
pub mod knot;
pub mod nurbs;
pub mod surface;

pub use bezier::Bezier;
pub use bspline::BSplineCurve;
pub use catmull_rom::CatmullRom;
pub use knot::{clamped_uniform_knots, de_boor4, domain, find_span};
pub use nurbs::NurbsCurve;
pub use surface::NurbsSurface;
