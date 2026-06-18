//! L-systems: string rewriting plus a 3D turtle interpreter.
//!
//! Define an [`LSystem`] (axiom + production rules), [`LSystem::expand`] it for
//! a number of iterations (deterministic or stochastic), then [`interpret`] the
//! resulting string as turtle graphics to get drawable [`Segment`]s.
//!
//! ```
//! use toolkit_lsystem::{LSystem, interpret, TurtleConfig};
//!
//! // Koch-like rule on a forward symbol.
//! let sys = LSystem::new("F").rule('F', "F+F-F");
//! let expanded = sys.expand(2);
//! let segments = interpret(&expanded, &TurtleConfig::default());
//! assert!(!segments.is_empty());
//! ```

pub mod system;
pub mod turtle;

pub use system::{LSystem, Production};
pub use turtle::{interpret, Segment, TurtleConfig};
