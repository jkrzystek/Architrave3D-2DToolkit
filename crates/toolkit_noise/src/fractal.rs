//! Fractal Brownian motion (fBm): sum several octaves of a base noise at
//! increasing frequency and decreasing amplitude for natural detail.

use serde::{Deserialize, Serialize};

use crate::perlin::Noise;

/// Which base noise an [`Fbm`] layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoiseKind {
    Perlin,
    Simplex,
    Value,
}

impl Noise {
    /// Sample the chosen base noise in 2D.
    pub fn sample2(&self, kind: NoiseKind, x: f32, y: f32) -> f32 {
        match kind {
            NoiseKind::Perlin => self.perlin2(x, y),
            NoiseKind::Simplex => self.simplex2(x, y),
            NoiseKind::Value => self.value2(x, y),
        }
    }
}

/// Fractal noise parameters.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Fbm {
    pub octaves: u32,
    /// Frequency multiplier per octave (typically 2.0).
    pub lacunarity: f32,
    /// Amplitude multiplier per octave (typically 0.5).
    pub gain: f32,
    /// Base frequency.
    pub frequency: f32,
    pub kind: NoiseKind,
}

impl Default for Fbm {
    fn default() -> Self {
        Self {
            octaves: 5,
            lacunarity: 2.0,
            gain: 0.5,
            frequency: 1.0,
            kind: NoiseKind::Perlin,
        }
    }
}

impl Fbm {
    pub fn new(kind: NoiseKind) -> Self {
        Self {
            kind,
            ..Default::default()
        }
    }

    /// Sample fBm in 2D. Output is normalised back to roughly the base noise
    /// range (≈ `[-1, 1]`) regardless of octave count.
    pub fn sample2(&self, noise: &Noise, x: f32, y: f32) -> f32 {
        let mut freq = self.frequency;
        let mut amp = 1.0;
        let mut sum = 0.0;
        let mut total_amp = 0.0;
        for _ in 0..self.octaves.max(1) {
            sum += noise.sample2(self.kind, x * freq, y * freq) * amp;
            total_amp += amp;
            freq *= self.lacunarity;
            amp *= self.gain;
        }
        if total_amp > 0.0 {
            sum / total_amp
        } else {
            0.0
        }
    }

    /// Ridged fBm: `1 - |noise|` per octave, producing sharp ridges (mountains).
    pub fn ridged2(&self, noise: &Noise, x: f32, y: f32) -> f32 {
        let mut freq = self.frequency;
        let mut amp = 1.0;
        let mut sum = 0.0;
        let mut total_amp = 0.0;
        for _ in 0..self.octaves.max(1) {
            let n = 1.0 - noise.sample2(self.kind, x * freq, y * freq).abs();
            sum += n * n * amp;
            total_amp += amp;
            freq *= self.lacunarity;
            amp *= self.gain;
        }
        if total_amp > 0.0 {
            sum / total_amp
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fbm_within_range() {
        let noise = Noise::new(1);
        let fbm = Fbm::new(NoiseKind::Perlin);
        for i in 0..200 {
            let v = fbm.sample2(&noise, i as f32 * 0.1, i as f32 * 0.07);
            assert!(v >= -1.2 && v <= 1.2, "out of range: {v}");
        }
    }

    #[test]
    fn more_octaves_add_detail() {
        let noise = Noise::new(2);
        let low = Fbm {
            octaves: 1,
            ..Fbm::new(NoiseKind::Perlin)
        };
        let high = Fbm {
            octaves: 6,
            ..Fbm::new(NoiseKind::Perlin)
        };
        // The two should generally disagree (extra octaves change the value).
        let a = low.sample2(&noise, 3.3, 4.4);
        let b = high.sample2(&noise, 3.3, 4.4);
        assert!((a - b).abs() > 1e-4);
    }

    #[test]
    fn ridged_is_non_negative() {
        let noise = Noise::new(3);
        let fbm = Fbm::new(NoiseKind::Simplex);
        for i in 0..100 {
            let v = fbm.ridged2(&noise, i as f32 * 0.2, 1.0);
            assert!(v >= 0.0, "ridged went negative: {v}");
        }
    }

    #[test]
    fn deterministic() {
        let noise = Noise::new(9);
        let fbm = Fbm::default();
        assert_eq!(
            fbm.sample2(&noise, 1.1, 2.2),
            fbm.sample2(&noise, 1.1, 2.2)
        );
    }
}
