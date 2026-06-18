//! A playback head that advances a time cursor over a fixed duration, with
//! looping, speed, and the usual transport controls.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AnimationPlayer {
    /// Current playback time in seconds.
    pub time: f32,
    /// Total length of the animation in seconds.
    pub duration: f32,
    /// Playback rate multiplier (negative plays backward).
    pub speed: f32,
    pub looping: bool,
    pub playing: bool,
}

impl AnimationPlayer {
    pub fn new(duration: f32) -> Self {
        Self {
            time: 0.0,
            duration: duration.max(0.0),
            speed: 1.0,
            looping: false,
            playing: true,
        }
    }

    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Advance the cursor by `dt` seconds (scaled by `speed`). At the end a
    /// looping player wraps; otherwise it clamps and stops.
    pub fn update(&mut self, dt: f32) {
        if !self.playing || self.duration <= 0.0 {
            return;
        }
        self.time += dt * self.speed;
        if self.looping {
            self.time = self.time.rem_euclid(self.duration);
        } else if self.time >= self.duration {
            self.time = self.duration;
            self.playing = false;
        } else if self.time < 0.0 {
            self.time = 0.0;
            self.playing = false;
        }
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Stop and rewind to the start.
    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
    }

    /// Jump to a specific time (clamped to the duration).
    pub fn seek(&mut self, time: f32) {
        self.time = time.clamp(0.0, self.duration);
    }

    /// Normalised progress in `[0, 1]`.
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            0.0
        } else {
            self.time / self.duration
        }
    }

    pub fn finished(&self) -> bool {
        !self.looping && !self.playing && self.time >= self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_time() {
        let mut p = AnimationPlayer::new(10.0);
        p.update(2.0);
        assert!((p.time - 2.0).abs() < 1e-6);
        assert!((p.progress() - 0.2).abs() < 1e-6);
    }

    #[test]
    fn clamps_and_stops_at_end() {
        let mut p = AnimationPlayer::new(1.0);
        p.update(5.0);
        assert_eq!(p.time, 1.0);
        assert!(!p.playing);
        assert!(p.finished());
    }

    #[test]
    fn loops_wrap_around() {
        let mut p = AnimationPlayer::new(1.0).looping(true);
        p.update(1.25);
        assert!((p.time - 0.25).abs() < 1e-5);
        assert!(p.playing);
        assert!(!p.finished());
    }

    #[test]
    fn speed_scales_advance() {
        let mut p = AnimationPlayer::new(10.0);
        p.speed = 2.0;
        p.update(1.0);
        assert!((p.time - 2.0).abs() < 1e-6);
    }

    #[test]
    fn transport_controls() {
        let mut p = AnimationPlayer::new(5.0);
        p.seek(3.0);
        assert_eq!(p.time, 3.0);
        p.pause();
        p.update(1.0);
        assert_eq!(p.time, 3.0); // paused: no advance
        p.play();
        p.update(1.0);
        assert!((p.time - 4.0).abs() < 1e-6);
        p.stop();
        assert_eq!(p.time, 0.0);
        assert!(!p.playing);
    }

    #[test]
    fn seek_clamps() {
        let mut p = AnimationPlayer::new(5.0);
        p.seek(100.0);
        assert_eq!(p.time, 5.0);
        p.seek(-10.0);
        assert_eq!(p.time, 0.0);
    }
}
