use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A single high-frequency input sample capturing pointer state at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InputSample {
    pub position: Vec2,
    pub pressure: f32,
    pub tilt: Vec2,
    pub timestamp_ms: f64,
    pub velocity: Vec2,
}

/// Ring buffer of [`InputSample`]s for high-frequency input telemetry.
///
/// When the buffer is full, the oldest sample is overwritten.
#[derive(Debug, Clone)]
pub struct InputBuffer {
    samples: Vec<Option<InputSample>>,
    /// Write index (points to the next slot to write).
    head: usize,
    /// Number of valid samples currently stored.
    len: usize,
}

impl InputBuffer {
    /// Create a new buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "InputBuffer capacity must be > 0");
        Self {
            samples: vec![None; capacity],
            head: 0,
            len: 0,
        }
    }

    /// Push a new sample into the ring buffer.
    pub fn push(&mut self, sample: InputSample) {
        self.samples[self.head] = Some(sample);
        self.head = (self.head + 1) % self.capacity();
        if self.len < self.capacity() {
            self.len += 1;
        }
    }

    /// Return the most recently pushed sample, or `None` if empty.
    pub fn latest(&self) -> Option<&InputSample> {
        if self.len == 0 {
            return None;
        }
        let idx = if self.head == 0 {
            self.capacity() - 1
        } else {
            self.head - 1
        };
        self.samples[idx].as_ref()
    }

    /// Return all samples with `timestamp_ms >= since`, ordered oldest-first.
    pub fn samples_since(&self, since: f64) -> Vec<&InputSample> {
        self.iter_oldest_first()
            .filter(|s| s.timestamp_ms >= since)
            .collect()
    }

    /// Average velocity over the last `n` samples (or fewer if the buffer
    /// contains fewer). Returns `Vec2::ZERO` if the buffer is empty.
    pub fn average_velocity(&self, n: usize) -> Vec2 {
        let recent: Vec<&InputSample> = self.iter_newest_first().take(n).collect();
        if recent.is_empty() {
            return Vec2::ZERO;
        }
        let sum: Vec2 = recent.iter().map(|s| s.velocity).sum();
        sum / recent.len() as f32
    }

    /// Average pressure over the last `n` samples (or fewer if the buffer
    /// contains fewer). Returns `0.0` if the buffer is empty.
    pub fn average_pressure(&self, n: usize) -> f32 {
        let recent: Vec<&InputSample> = self.iter_newest_first().take(n).collect();
        if recent.is_empty() {
            return 0.0;
        }
        let sum: f32 = recent.iter().map(|s| s.pressure).sum();
        sum / recent.len() as f32
    }

    /// Clear all stored samples.
    pub fn clear(&mut self) {
        for slot in self.samples.iter_mut() {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
    }

    /// The maximum number of samples this buffer can hold.
    pub fn capacity(&self) -> usize {
        self.samples.len()
    }

    /// The number of valid samples currently stored.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if no samples are stored.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    // ------ internal iterators ------

    /// Iterate from oldest to newest.
    fn iter_oldest_first(&self) -> impl Iterator<Item = &InputSample> {
        let cap = self.capacity();
        let start = if self.len < cap {
            0
        } else {
            self.head // oldest element when buffer is full
        };
        let len = self.len;
        (0..len).filter_map(move |i| {
            let idx = (start + i) % cap;
            self.samples[idx].as_ref()
        })
    }

    /// Iterate from newest to oldest.
    fn iter_newest_first(&self) -> impl Iterator<Item = &InputSample> {
        let cap = self.capacity();
        let len = self.len;
        (0..len).filter_map(move |i| {
            let idx = if self.head == 0 {
                cap - 1 - i
            } else {
                (self.head + cap - 1 - i) % cap
            };
            self.samples[idx].as_ref()
        })
    }
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self::new(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(ts: f64, x: f32, pressure: f32) -> InputSample {
        InputSample {
            position: Vec2::new(x, 0.0),
            pressure,
            tilt: Vec2::ZERO,
            timestamp_ms: ts,
            velocity: Vec2::new(x, 0.0),
        }
    }

    #[test]
    fn empty_buffer() {
        let buf = InputBuffer::new(4);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(buf.latest().is_none());
        assert_eq!(buf.average_velocity(5), Vec2::ZERO);
        assert_eq!(buf.average_pressure(5), 0.0);
    }

    #[test]
    fn push_and_latest() {
        let mut buf = InputBuffer::new(4);
        buf.push(make_sample(1.0, 10.0, 0.5));
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.latest().unwrap().timestamp_ms, 1.0);

        buf.push(make_sample(2.0, 20.0, 0.6));
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.latest().unwrap().timestamp_ms, 2.0);
    }

    #[test]
    fn overflow_wraps_around() {
        let mut buf = InputBuffer::new(3);
        for i in 0..5 {
            buf.push(make_sample(i as f64, i as f32, 1.0));
        }
        // Capacity is 3, so only the last 3 should remain.
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.latest().unwrap().timestamp_ms, 4.0);

        // Oldest-first should be [2, 3, 4]
        let all: Vec<f64> = buf.iter_oldest_first().map(|s| s.timestamp_ms).collect();
        assert_eq!(all, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn samples_since_filters_correctly() {
        let mut buf = InputBuffer::new(8);
        for i in 0..6 {
            buf.push(make_sample(i as f64 * 10.0, i as f32, 1.0));
        }
        // timestamps: 0, 10, 20, 30, 40, 50
        let recent = buf.samples_since(25.0);
        let ts: Vec<f64> = recent.iter().map(|s| s.timestamp_ms).collect();
        assert_eq!(ts, vec![30.0, 40.0, 50.0]);
    }

    #[test]
    fn average_velocity_computes_correctly() {
        let mut buf = InputBuffer::new(8);
        buf.push(make_sample(0.0, 2.0, 1.0));
        buf.push(make_sample(1.0, 4.0, 1.0));
        buf.push(make_sample(2.0, 6.0, 1.0));
        // Last 2 velocities: (6,0) and (4,0) => avg (5, 0)
        let avg = buf.average_velocity(2);
        assert!((avg.x - 5.0).abs() < 1e-6);
        assert!((avg.y).abs() < 1e-6);
    }

    #[test]
    fn average_pressure_computes_correctly() {
        let mut buf = InputBuffer::new(8);
        buf.push(make_sample(0.0, 0.0, 0.2));
        buf.push(make_sample(1.0, 0.0, 0.4));
        buf.push(make_sample(2.0, 0.0, 0.6));
        // Last 2 pressures: 0.6 and 0.4 => avg 0.5
        let avg = buf.average_pressure(2);
        assert!((avg - 0.5).abs() < 1e-6);
    }

    #[test]
    fn average_with_fewer_samples_than_requested() {
        let mut buf = InputBuffer::new(8);
        buf.push(make_sample(0.0, 3.0, 0.9));
        // Ask for 10 but only have 1
        let avg = buf.average_velocity(10);
        assert!((avg.x - 3.0).abs() < 1e-6);
    }

    #[test]
    fn clear_resets_buffer() {
        let mut buf = InputBuffer::new(4);
        buf.push(make_sample(1.0, 1.0, 1.0));
        buf.push(make_sample(2.0, 2.0, 1.0));
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(buf.latest().is_none());
    }

    #[test]
    fn default_capacity_is_256() {
        let buf = InputBuffer::default();
        assert_eq!(buf.capacity(), 256);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn zero_capacity_panics() {
        InputBuffer::new(0);
    }

    #[test]
    fn newest_first_iteration_order() {
        let mut buf = InputBuffer::new(4);
        buf.push(make_sample(10.0, 1.0, 1.0));
        buf.push(make_sample(20.0, 2.0, 1.0));
        buf.push(make_sample(30.0, 3.0, 1.0));
        let ts: Vec<f64> = buf.iter_newest_first().map(|s| s.timestamp_ms).collect();
        assert_eq!(ts, vec![30.0, 20.0, 10.0]);
    }
}
