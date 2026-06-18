//! Keyframe animation.
//!
//! Build [`Track`]s of timed values (any [`Animatable`] — `f32`, `Vec3`,
//! `Quat`), group three of them into a [`TransformAnimation`], and advance a
//! [`AnimationPlayer`] to drive [`toolkit_scene`] node transforms over time.
//! Per-keyframe easing comes from [`toolkit_easing`].
//!
//! ```
//! use glam::Vec3;
//! use toolkit_anim::{TransformAnimation, AnimationPlayer};
//!
//! let mut anim = TransformAnimation::new();
//! anim.translation.add(0.0, Vec3::ZERO).add(2.0, Vec3::new(10.0, 0.0, 0.0));
//!
//! let mut player = AnimationPlayer::new(anim.duration());
//! player.update(1.0);                       // 1s in
//! let t = anim.sample(player.time);         // halfway -> x = 5
//! assert!((t.translation.x - 5.0).abs() < 1e-5);
//! ```

pub mod player;
pub mod track;
pub mod transform_anim;

pub use player::AnimationPlayer;
pub use track::{Animatable, Keyframe, Track};
pub use transform_anim::{apply_to_node, TransformAnimation};

// Re-export the easing vocabulary so callers don't need a second `use`.
pub use toolkit_easing::Easing;
