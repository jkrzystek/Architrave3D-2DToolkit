//! 2D simplex noise (Gustavson), seeded by [`Noise`]'s permutation table.
//! Simplex avoids the directional grid artifacts of Perlin noise.

use crate::perlin::Noise;

const F2: f32 = 0.366_025_42; // 0.5 * (sqrt(3) - 1)
const G2: f32 = 0.211_324_87; // (3 - sqrt(3)) / 6

/// 8 unit gradient directions.
fn grad(hash: u8, x: f32, y: f32) -> f32 {
    const S: f32 = std::f32::consts::FRAC_1_SQRT_2;
    let (gx, gy) = match hash & 7 {
        0 => (1.0, 0.0),
        1 => (-1.0, 0.0),
        2 => (0.0, 1.0),
        3 => (0.0, -1.0),
        4 => (S, S),
        5 => (-S, S),
        6 => (S, -S),
        _ => (-S, -S),
    };
    gx * x + gy * y
}

impl Noise {
    /// 2D simplex noise, output ≈ `[-1, 1]`.
    pub fn simplex2(&self, xin: f32, yin: f32) -> f32 {
        // Skew the input space to the simplex grid.
        let s = (xin + yin) * F2;
        let i = (xin + s).floor();
        let j = (yin + s).floor();
        let t = (i + j) * G2;
        let x0 = xin - (i - t);
        let y0 = yin - (j - t);

        // Which simplex (triangle) are we in?
        let (i1, j1) = if x0 > y0 { (1.0, 0.0) } else { (0.0, 1.0) };

        let x1 = x0 - i1 + G2;
        let y1 = y0 - j1 + G2;
        let x2 = x0 - 1.0 + 2.0 * G2;
        let y2 = y0 - 1.0 + 2.0 * G2;

        let ii = i as i32;
        let jj = j as i32;
        let g0 = self.p(ii + self.p(jj) as i32);
        let g1 = self.p(ii + i1 as i32 + self.p(jj + j1 as i32) as i32);
        let g2 = self.p(ii + 1 + self.p(jj + 1) as i32) as u8;

        let contrib = |x: f32, y: f32, gh: u8| -> f32 {
            let t = 0.5 - x * x - y * y;
            if t < 0.0 {
                0.0
            } else {
                let t2 = t * t;
                t2 * t2 * grad(gh, x, y)
            }
        };

        let n = contrib(x0, y0, g0) + contrib(x1, y1, g1) + contrib(x2, y2, g2);
        // Scale to roughly fill [-1, 1].
        70.0 * n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a = Noise::new(5);
        let b = Noise::new(5);
        assert_eq!(a.simplex2(1.5, 2.5), b.simplex2(1.5, 2.5));
    }

    #[test]
    fn within_reasonable_range() {
        let n = Noise::new(1);
        let mut x = 0.0;
        while x < 40.0 {
            let mut y = 0.0;
            while y < 40.0 {
                let v = n.simplex2(x * 0.13, y * 0.13);
                assert!(v >= -1.5 && v <= 1.5, "out of range: {v}");
                y += 1.0;
            }
            x += 1.0;
        }
    }

    #[test]
    fn is_continuous() {
        let n = Noise::new(2);
        let a = n.simplex2(3.0, 4.0);
        let b = n.simplex2(3.002, 4.0);
        assert!((a - b).abs() < 0.05);
    }

    #[test]
    fn has_variation() {
        // Not a constant function.
        let n = Noise::new(7);
        let a = n.simplex2(0.2, 0.2);
        let b = n.simplex2(10.7, 4.3);
        assert!((a - b).abs() > 1e-3);
    }
}
