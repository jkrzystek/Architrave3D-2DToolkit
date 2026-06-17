use serde::{Deserialize, Serialize};

use crate::easing::{ease, Easing};

/// How a tween behaves when it reaches the end of its duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    /// Stop at the end.
    Once,
    /// Jump back to the start and play again.
    Loop,
    /// Reverse direction at each end (ping-pong).
    PingPong,
}

/// A time-driven tween producing an eased progress value in `[0, 1]`.
///
/// It is value-agnostic: feed `progress()` into [`lerp`](crate::lerp) (or
/// `glam`'s `Vec*::lerp`) to interpolate whatever you like. This keeps the
/// crate free of any math dependency.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tween {
    pub duration: f32,
    pub elapsed: f32,
    pub easing: Easing,
    pub repeat: Repeat,
    forward: bool,
}

impl Tween {
    pub fn new(duration: f32, easing: Easing) -> Self {
        Self {
            duration: duration.max(1e-6),
            elapsed: 0.0,
            easing,
            repeat: Repeat::Once,
            forward: true,
        }
    }

    pub fn with_repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// Advance time by `dt` and return the current eased progress.
    pub fn update(&mut self, dt: f32) -> f32 {
        self.elapsed += dt;
        if self.elapsed >= self.duration {
            match self.repeat {
                Repeat::Once => self.elapsed = self.duration,
                Repeat::Loop => self.elapsed %= self.duration,
                Repeat::PingPong => {
                    self.elapsed %= self.duration;
                    self.forward = !self.forward;
                }
            }
        }
        self.progress()
    }

    /// The current eased progress without advancing time.
    pub fn progress(&self) -> f32 {
        let linear = (self.elapsed / self.duration).clamp(0.0, 1.0);
        let linear = if self.forward { linear } else { 1.0 - linear };
        ease(self.easing, linear)
    }

    /// True once a non-repeating tween has reached its end.
    pub fn finished(&self) -> bool {
        self.repeat == Repeat::Once && self.elapsed >= self.duration
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.forward = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_tween_midpoint() {
        let mut t = Tween::new(2.0, Easing::Linear);
        let p = t.update(1.0);
        assert!((p - 0.5).abs() < 1e-5);
        assert!(!t.finished());
    }

    #[test]
    fn once_clamps_and_finishes() {
        let mut t = Tween::new(1.0, Easing::Linear);
        t.update(5.0);
        assert!(t.finished());
        assert!((t.progress() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn loop_wraps() {
        let mut t = Tween::new(1.0, Easing::Linear).with_repeat(Repeat::Loop);
        t.update(1.25);
        assert!(!t.finished());
        assert!((t.progress() - 0.25).abs() < 1e-4);
    }

    #[test]
    fn pingpong_reverses() {
        let mut t = Tween::new(1.0, Easing::Linear).with_repeat(Repeat::PingPong);
        t.update(1.5); // past the end once -> now going backward
        let p = t.progress();
        // Halfway back from the end.
        assert!((p - 0.5).abs() < 1e-4, "p = {p}");
    }

    #[test]
    fn reset_returns_to_start() {
        let mut t = Tween::new(1.0, Easing::Linear);
        t.update(0.7);
        t.reset();
        assert_eq!(t.elapsed, 0.0);
        assert!((t.progress()).abs() < 1e-5);
    }
}
