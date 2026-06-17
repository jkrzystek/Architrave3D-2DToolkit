//! Gradient (Perlin) and value noise, seeded by a permutation table.

use serde::{Deserialize, Serialize};

/// Integer bit-mix hash (used to build the permutation table and for cells).
pub(crate) fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// A seeded noise source providing Perlin and value noise in 2D/3D. Output is
/// approximately in `[-1, 1]`. The [`crate::simplex`] and [`crate::worley`]
/// functions take the same `seed` for consistency.
#[derive(Clone, Serialize, Deserialize)]
pub struct Noise {
    pub seed: u32,
    #[serde(skip, default = "default_perm")]
    perm: [u8; 512],
}

fn default_perm() -> [u8; 512] {
    build_perm(0)
}

fn build_perm(seed: u32) -> [u8; 512] {
    let mut p: [u8; 256] = [0; 256];
    for (i, slot) in p.iter_mut().enumerate() {
        *slot = i as u8;
    }
    // Deterministic Fisher-Yates driven by the seed.
    let mut state = hash_u32(seed ^ 0x9e37_79b9);
    for i in (1..256).rev() {
        state = hash_u32(state);
        let j = (state % (i as u32 + 1)) as usize;
        p.swap(i, j);
    }
    let mut perm = [0u8; 512];
    for i in 0..512 {
        perm[i] = p[i & 255];
    }
    perm
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

fn grad2(hash: u8, x: f32, y: f32) -> f32 {
    // 8 gradient directions.
    match hash & 7 {
        0 => x + y,
        1 => x - y,
        2 => -x + y,
        3 => -x - y,
        4 => x,
        5 => -x,
        6 => y,
        _ => -y,
    }
}

fn grad3(hash: u8, x: f32, y: f32, z: f32) -> f32 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            perm: build_perm(seed),
        }
    }

    #[inline]
    pub(crate) fn p(&self, i: i32) -> u8 {
        self.perm[(i & 255) as usize]
    }

    /// 2D Perlin noise, output ≈ `[-1, 1]`.
    pub fn perlin2(&self, x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let u = fade(xf);
        let v = fade(yf);

        let aa = self.p(self.p(xi) as i32 + yi) as u8;
        let ab = self.p(self.p(xi) as i32 + yi + 1) as u8;
        let ba = self.p(self.p(xi + 1) as i32 + yi) as u8;
        let bb = self.p(self.p(xi + 1) as i32 + yi + 1) as u8;

        let x1 = lerp(u, grad2(aa, xf, yf), grad2(ba, xf - 1.0, yf));
        let x2 = lerp(u, grad2(ab, xf, yf - 1.0), grad2(bb, xf - 1.0, yf - 1.0));
        // Scale ~1.4 to better fill [-1,1].
        lerp(v, x1, x2) * 1.4
    }

    /// 3D Perlin noise, output ≈ `[-1, 1]`.
    pub fn perlin3(&self, x: f32, y: f32, z: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let zi = z.floor() as i32;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let zf = z - z.floor();
        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);

        let a = self.p(xi) as i32 + yi;
        let aa = self.p(a) as i32 + zi;
        let ab = self.p(a + 1) as i32 + zi;
        let b = self.p(xi + 1) as i32 + yi;
        let ba = self.p(b) as i32 + zi;
        let bb = self.p(b + 1) as i32 + zi;

        let x1 = lerp(
            u,
            grad3(self.p(aa), xf, yf, zf),
            grad3(self.p(ba), xf - 1.0, yf, zf),
        );
        let x2 = lerp(
            u,
            grad3(self.p(ab), xf, yf - 1.0, zf),
            grad3(self.p(bb), xf - 1.0, yf - 1.0, zf),
        );
        let y1 = lerp(v, x1, x2);

        let x3 = lerp(
            u,
            grad3(self.p(aa + 1), xf, yf, zf - 1.0),
            grad3(self.p(ba + 1), xf - 1.0, yf, zf - 1.0),
        );
        let x4 = lerp(
            u,
            grad3(self.p(ab + 1), xf, yf - 1.0, zf - 1.0),
            grad3(self.p(bb + 1), xf - 1.0, yf - 1.0, zf - 1.0),
        );
        let y2 = lerp(v, x3, x4);

        lerp(w, y1, y2)
    }

    /// 2D value noise (smooth interpolation of per-lattice random values),
    /// output ≈ `[-1, 1]`.
    pub fn value2(&self, x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let u = fade(xf);
        let v = fade(yf);

        let lattice = |gx: i32, gy: i32| -> f32 {
            let h = self.p(self.p(gx) as i32 + gy);
            (h as f32 / 127.5) - 1.0
        };

        let x1 = lerp(u, lattice(xi, yi), lattice(xi + 1, yi));
        let x2 = lerp(u, lattice(xi, yi + 1), lattice(xi + 1, yi + 1));
        lerp(v, x1, x2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_seed() {
        let a = Noise::new(10);
        let b = Noise::new(10);
        assert_eq!(a.perlin2(1.3, 2.7), b.perlin2(1.3, 2.7));
        assert_eq!(a.perlin3(1.0, 2.0, 3.0), b.perlin3(1.0, 2.0, 3.0));
    }

    #[test]
    fn perlin_within_range() {
        let n = Noise::new(1);
        let mut x = 0.0;
        while x < 50.0 {
            let mut y = 0.0;
            while y < 50.0 {
                let v = n.perlin2(x * 0.1, y * 0.1);
                assert!(v >= -1.2 && v <= 1.2, "out of range: {v}");
                y += 1.0;
            }
            x += 1.0;
        }
    }

    #[test]
    fn perlin_is_continuous() {
        let n = Noise::new(2);
        let a = n.perlin2(3.0, 4.0);
        let b = n.perlin2(3.001, 4.0);
        assert!((a - b).abs() < 0.05, "noise jumped: {a} vs {b}");
    }

    #[test]
    fn perlin_zero_at_lattice_is_small() {
        // Perlin noise is exactly 0 at integer lattice points.
        let n = Noise::new(3);
        assert!(n.perlin2(5.0, 7.0).abs() < 1e-5);
        assert!(n.perlin3(2.0, 3.0, 4.0).abs() < 1e-5);
    }

    #[test]
    fn different_seeds_differ() {
        let a = Noise::new(1);
        let b = Noise::new(2);
        assert_ne!(a.perlin2(0.5, 0.5), b.perlin2(0.5, 0.5));
    }
}
