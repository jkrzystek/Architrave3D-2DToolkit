//! Keyframe tracks: a sorted list of timed values sampled by interpolation.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use toolkit_easing::{ease, Easing};

/// A value that can be interpolated between keyframes.
pub trait Animatable: Copy {
    fn interpolate(a: Self, b: Self, t: f32) -> Self;
}

impl Animatable for f32 {
    fn interpolate(a: Self, b: Self, t: f32) -> Self {
        a + (b - a) * t
    }
}
impl Animatable for Vec3 {
    fn interpolate(a: Self, b: Self, t: f32) -> Self {
        a.lerp(b, t)
    }
}
impl Animatable for Quat {
    fn interpolate(a: Self, b: Self, t: f32) -> Self {
        // Shortest-path spherical interpolation.
        a.slerp(b, t)
    }
}

/// A single keyframe: a value at a time, with the easing used to interpolate
/// *into* it from the previous key.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
    pub easing: Easing,
}

/// A timeline of keyframes for one animated property.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Track<T> {
    keys: Vec<Keyframe<T>>,
}

impl<T: Animatable> Track<T> {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    /// Insert a linear keyframe (keys stay sorted by time).
    pub fn add(&mut self, time: f32, value: T) -> &mut Self {
        self.add_eased(time, value, Easing::Linear)
    }

    /// Insert a keyframe with a specific ease-in curve.
    pub fn add_eased(&mut self, time: f32, value: T, easing: Easing) -> &mut Self {
        let key = Keyframe { time, value, easing };
        let pos = self
            .keys
            .binary_search_by(|k| k.time.total_cmp(&time))
            .unwrap_or_else(|e| e);
        self.keys.insert(pos, key);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Time of the last keyframe (0 if empty).
    pub fn duration(&self) -> f32 {
        self.keys.last().map(|k| k.time).unwrap_or(0.0)
    }

    /// Sample the track at `time`. Holds the first/last value outside the range.
    /// Returns `None` only when the track is empty.
    pub fn sample(&self, time: f32) -> Option<T> {
        match self.keys.len() {
            0 => None,
            1 => Some(self.keys[0].value),
            _ => {
                let first = &self.keys[0];
                let last = &self.keys[self.keys.len() - 1];
                if time <= first.time {
                    return Some(first.value);
                }
                if time >= last.time {
                    return Some(last.value);
                }
                // Find the segment [i, i+1] containing `time`.
                let i = match self.keys.binary_search_by(|k| k.time.total_cmp(&time)) {
                    Ok(idx) => return Some(self.keys[idx].value),
                    Err(idx) => idx - 1,
                };
                let a = &self.keys[i];
                let b = &self.keys[i + 1];
                let span = (b.time - a.time).max(1e-9);
                let local = (time - a.time) / span;
                let eased = ease(b.easing, local);
                Some(T::interpolate(a.value, b.value, eased))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_track_returns_none() {
        let t: Track<f32> = Track::new();
        assert!(t.sample(0.5).is_none());
    }

    #[test]
    fn linear_sampling() {
        let mut t = Track::new();
        t.add(0.0, 0.0).add(2.0, 10.0);
        assert!((t.sample(1.0).unwrap() - 5.0).abs() < 1e-5);
        assert_eq!(t.duration(), 2.0);
    }

    #[test]
    fn holds_endpoints_outside_range() {
        let mut t = Track::new();
        t.add(1.0, 3.0).add(2.0, 9.0);
        assert_eq!(t.sample(0.0).unwrap(), 3.0);
        assert_eq!(t.sample(5.0).unwrap(), 9.0);
    }

    #[test]
    fn easing_changes_midpoint() {
        let mut lin = Track::new();
        lin.add(0.0, 0.0).add(1.0, 1.0);
        let mut eased = Track::new();
        eased
            .add_eased(0.0, 0.0, Easing::Linear)
            .add_eased(1.0, 1.0, Easing::QuadIn);
        // Quad ease-in is below linear at the midpoint.
        assert!(eased.sample(0.5).unwrap() < lin.sample(0.5).unwrap());
    }

    #[test]
    fn keys_stay_sorted_when_inserted_out_of_order() {
        let mut t = Track::new();
        t.add(2.0, 20.0).add(0.0, 0.0).add(1.0, 10.0);
        assert!((t.sample(0.5).unwrap() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn quaternion_slerp() {
        let mut t = Track::new();
        let a = Quat::IDENTITY;
        let b = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        t.add(0.0, a).add(1.0, b);
        let mid = t.sample(0.5).unwrap();
        // Halfway should rotate +X about 45 degrees.
        let rotated = mid * Vec3::X;
        let expected = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4) * Vec3::X;
        assert!((rotated - expected).length() < 1e-4);
    }
}
