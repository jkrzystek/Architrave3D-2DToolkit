//! Worley (cellular) noise: distance to the nearest randomly-placed feature
//! point. Great for stone, scales, cracks, and biome cells.

use crate::perlin::hash_u32;

fn cell_jitter(seed: u32, cx: i32, cy: i32) -> (f32, f32) {
    let h = hash_u32(
        seed ^ (cx as u32).wrapping_mul(0x1f1f_1f1f) ^ (cy as u32).wrapping_mul(0x27d4_eb2f),
    );
    let jx = (h & 0xffff) as f32 / 65535.0;
    let jy = ((h >> 16) & 0xffff) as f32 / 65535.0;
    (jx, jy)
}

fn cell_jitter3(seed: u32, cx: i32, cy: i32, cz: i32) -> (f32, f32, f32) {
    let base = seed
        ^ (cx as u32).wrapping_mul(0x1f1f_1f1f)
        ^ (cy as u32).wrapping_mul(0x27d4_eb2f)
        ^ (cz as u32).wrapping_mul(0x1656_67b1);
    let h = hash_u32(base);
    let h2 = hash_u32(h);
    let jx = (h & 0xffff) as f32 / 65535.0;
    let jy = ((h >> 16) & 0xffff) as f32 / 65535.0;
    let jz = (h2 & 0xffff) as f32 / 65535.0;
    (jx, jy, jz)
}

/// Distance to the nearest feature point (F1), 2D. Typically in `[0, ~1.4]`.
pub fn worley2(seed: u32, x: f32, y: f32) -> f32 {
    worley2_f2(seed, x, y).0
}

/// Distances to the nearest (F1) and second-nearest (F2) feature points, 2D.
/// `F2 - F1` gives the classic cellular "cracks" pattern.
pub fn worley2_f2(seed: u32, x: f32, y: f32) -> (f32, f32) {
    let cx = x.floor() as i32;
    let cy = y.floor() as i32;
    let mut f1 = f32::INFINITY;
    let mut f2 = f32::INFINITY;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (jx, jy) = cell_jitter(seed, cx + dx, cy + dy);
            let fx = (cx + dx) as f32 + jx;
            let fy = (cy + dy) as f32 + jy;
            let d2 = (fx - x) * (fx - x) + (fy - y) * (fy - y);
            if d2 < f1 {
                f2 = f1;
                f1 = d2;
            } else if d2 < f2 {
                f2 = d2;
            }
        }
    }
    (f1.sqrt(), f2.sqrt())
}

/// Distance to the nearest feature point (F1), 3D.
pub fn worley3(seed: u32, x: f32, y: f32, z: f32) -> f32 {
    let cx = x.floor() as i32;
    let cy = y.floor() as i32;
    let cz = z.floor() as i32;
    let mut f1 = f32::INFINITY;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (jx, jy, jz) = cell_jitter3(seed, cx + dx, cy + dy, cz + dz);
                let fx = (cx + dx) as f32 + jx;
                let fy = (cy + dy) as f32 + jy;
                let fz = (cz + dz) as f32 + jz;
                let d2 = (fx - x).powi(2) + (fy - y).powi(2) + (fz - z).powi(2);
                if d2 < f1 {
                    f1 = d2;
                }
            }
        }
    }
    f1.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(worley2(3, 1.4, 2.6), worley2(3, 1.4, 2.6));
        assert_eq!(worley3(3, 1.0, 2.0, 3.0), worley3(3, 1.0, 2.0, 3.0));
    }

    #[test]
    fn f1_non_negative_and_f2_ge_f1() {
        for i in 0..100 {
            let x = i as f32 * 0.37;
            let y = i as f32 * 0.51;
            let (f1, f2) = worley2_f2(7, x, y);
            assert!(f1 >= 0.0);
            assert!(f2 >= f1 - 1e-5);
        }
    }

    #[test]
    fn near_a_feature_point_is_small() {
        // The center of cell (0,0)'s jittered point should be within ~1 of it.
        let (jx, jy) = cell_jitter(7, 0, 0);
        let f1 = worley2(7, jx, jy);
        assert!(f1 < 1e-4, "f1 at feature point = {f1}");
    }

    #[test]
    fn different_seeds_differ() {
        assert_ne!(worley2(1, 0.5, 0.5), worley2(2, 0.5, 0.5));
    }
}
