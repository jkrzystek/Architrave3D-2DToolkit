//! Named, typed attribute channels bound to geometry domains.
//!
//! This is the procedural-attribute backbone: instead of hard-coding which data
//! a vertex carries, geometry and tools attach arbitrary named channels
//! ([`Attribute`]) on a [`Domain`] (point, vertex, edge, face, primitive, or a
//! single global detail value). The same machinery stores sculpt masks, paint
//! weights, soft-selection falloff, per-element simulation state, and extra UV
//! sets — anything that flows through a node graph.
//!
//! - [`Attribute`] — one columnar channel of a single [`AttributeType`].
//! - [`AttributeSet`] — all channels for one domain, kept the same length.
//! - [`AttributeStore`] — one set per domain, the bundle geometry carries.
//!
//! ```
//! use toolkit_attributes::{AttributeStore, AttributeType, Domain};
//! use glam::Vec3;
//!
//! let mut store = AttributeStore::new();
//! let cd = store.create(Domain::Point, 3, "Cd", AttributeType::Color);
//! cd.set_vec4(0, glam::Vec4::new(1.0, 0.0, 0.0, 1.0));
//!
//! // A second channel on the same domain reuses the element count.
//! store.create(Domain::Point, 0, "mask", AttributeType::Float);
//! assert_eq!(store.get(Domain::Point, "mask").unwrap().len(), 3);
//! # let _ = Vec3::ZERO;
//! ```

pub mod set;
pub mod store;
pub mod types;

pub use set::AttributeSet;
pub use store::AttributeStore;
pub use types::{Attribute, AttributeData, AttributeType, Domain};
