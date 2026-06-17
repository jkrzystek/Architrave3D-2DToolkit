use serde::{Deserialize, Serialize};

/// A small, fast, deterministic PRNG (PCG-XSH-RR, 32-bit output).
///
/// Deterministic for a given seed across platforms, which is what reproducible
/// procedural generation needs. Not cryptographically secure.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
    inc: u64,
}

const MULTIPLIER: u64 = 6364136223846793005;

impl Rng {
    /// Seed from a single integer (stream defaults to a fixed sequence).
    pub fn seed_from_u64(seed: u64) -> Self {
        Self::seed_with_stream(seed, 0xda3e_39cb_94b9_5bdb)
    }

    /// Seed with an explicit stream selector, so two RNGs with the same seed but
    /// different streams produce independent sequences.
    pub fn seed_with_stream(seed: u64, stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// Raw 32-bit output.
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Raw 64-bit output (two draws).
    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }

    /// Uniform `f32` in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        // 24 bits of mantissa precision.
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform `f64` in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform `f32` in `[min, max)`.
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }

    /// Uniform integer in `[min, max)`. Returns `min` if the range is empty.
    pub fn range_u32(&mut self, min: u32, max: u32) -> u32 {
        if max <= min {
            return min;
        }
        let span = max - min;
        // Rejection sampling to remove modulo bias.
        let threshold = span.wrapping_neg() % span;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return min + r % span;
            }
        }
    }

    /// Uniform integer in `[min, max)` (signed).
    pub fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        if max <= min {
            return min;
        }
        let span = (max - min) as u32;
        min + self.range_u32(0, span) as i32
    }

    /// `true` with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }

    /// A standard-normal sample (mean 0, std 1) via Box-Muller.
    pub fn next_normal(&mut self) -> f32 {
        // Avoid log(0).
        let u1 = (self.next_f32()).max(1e-7);
        let u2 = self.next_f32();
        (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
    }

    /// A normal sample with the given mean and standard deviation.
    pub fn normal(&mut self, mean: f32, std_dev: f32) -> f32 {
        mean + std_dev * self.next_normal()
    }

    /// Fisher-Yates in-place shuffle.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        let n = slice.len();
        for i in (1..n).rev() {
            let j = self.range_u32(0, (i + 1) as u32) as usize;
            slice.swap(i, j);
        }
    }

    /// Pick a random element by reference.
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            Some(&slice[self.range_u32(0, slice.len() as u32) as usize])
        }
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::seed_from_u64(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_seed() {
        let mut a = Rng::seed_from_u64(42);
        let mut b = Rng::seed_from_u64(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::seed_from_u64(1);
        let mut b = Rng::seed_from_u64(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn f32_in_unit_range() {
        let mut r = Rng::seed_from_u64(7);
        for _ in 0..10_000 {
            let v = r.next_f32();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn range_u32_respects_bounds() {
        let mut r = Rng::seed_from_u64(7);
        for _ in 0..10_000 {
            let v = r.range_u32(10, 20);
            assert!((10..20).contains(&v));
        }
    }

    #[test]
    fn normal_mean_is_close() {
        let mut r = Rng::seed_from_u64(3);
        let n = 50_000;
        let sum: f32 = (0..n).map(|_| r.normal(5.0, 2.0)).sum();
        let mean = sum / n as f32;
        assert!((mean - 5.0).abs() < 0.1, "mean = {mean}");
    }

    #[test]
    fn shuffle_preserves_elements() {
        let mut r = Rng::seed_from_u64(9);
        let mut data: Vec<i32> = (0..50).collect();
        let original = data.clone();
        r.shuffle(&mut data);
        let mut sorted = data.clone();
        sorted.sort();
        assert_eq!(sorted, original);
        // Extremely unlikely to be identical after shuffle.
        assert_ne!(data, original);
    }

    #[test]
    fn choose_returns_member() {
        let mut r = Rng::seed_from_u64(1);
        let data = [1, 2, 3, 4];
        let c = r.choose(&data).unwrap();
        assert!(data.contains(c));
        let empty: [i32; 0] = [];
        assert!(r.choose(&empty).is_none());
    }
}
