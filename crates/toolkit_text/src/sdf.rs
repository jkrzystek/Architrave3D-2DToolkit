//! Turn a binary coverage mask (from any glyph rasterizer) into a signed
//! distance field, so glyphs stay crisp at any scale.
//!
//! Distances come from the 8SSEDT (8-point sequential signed Euclidean distance
//! transform): two linear passes that propagate the offset to the nearest seed
//! pixel. We run it once for inside seeds and once for outside, then subtract.

use toolkit_image::Image;

#[derive(Clone, Copy)]
struct Cell {
    dx: i32,
    dy: i32,
}

impl Cell {
    #[inline]
    fn dist_sq(self) -> i32 {
        self.dx * self.dx + self.dy * self.dy
    }
}

const EMPTY: Cell = Cell { dx: 16384, dy: 16384 };
const SEED: Cell = Cell { dx: 0, dy: 0 };

/// Euclidean distance from every pixel to the nearest pixel where `is_seed` is
/// true, computed with 8SSEDT.
fn distance_transform(width: usize, height: usize, is_seed: impl Fn(usize) -> bool) -> Vec<f32> {
    let mut grid = vec![EMPTY; width * height];
    for (i, cell) in grid.iter_mut().enumerate() {
        if is_seed(i) {
            *cell = SEED;
        }
    }

    let w = width as i32;
    let h = height as i32;
    let at = |x: i32, y: i32| (y as usize) * width + x as usize;
    let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h;

    // Try to improve cell (x,y) using neighbour (x+ox, y+oy).
    let relax = |grid: &mut [Cell], x: i32, y: i32, ox: i32, oy: i32| {
        if !in_bounds(x + ox, y + oy) {
            return;
        }
        let mut other = grid[at(x + ox, y + oy)];
        other.dx += ox;
        other.dy += oy;
        let here = &mut grid[at(x, y)];
        if other.dist_sq() < here.dist_sq() {
            *here = other;
        }
    };

    // Pass 1: top→bottom.
    for y in 0..h {
        for x in 0..w {
            relax(&mut grid, x, y, -1, 0);
            relax(&mut grid, x, y, 0, -1);
            relax(&mut grid, x, y, -1, -1);
            relax(&mut grid, x, y, 1, -1);
        }
        for x in (0..w).rev() {
            relax(&mut grid, x, y, 1, 0);
        }
    }
    // Pass 2: bottom→top.
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            relax(&mut grid, x, y, 1, 0);
            relax(&mut grid, x, y, 0, 1);
            relax(&mut grid, x, y, 1, 1);
            relax(&mut grid, x, y, -1, 1);
        }
        for x in 0..w {
            relax(&mut grid, x, y, -1, 0);
        }
    }

    grid.iter().map(|c| (c.dist_sq() as f32).sqrt()).collect()
}

/// Build a single-channel SDF image from a coverage mask.
///
/// `coverage` holds one alpha byte per pixel (`>= 128` is inside the glyph).
/// `spread` is the distance in pixels mapped to the `[0, 1]` edge range — a
/// larger spread supports thicker outlines and softer scaling. The result is
/// grayscale (R=G=B=distance, A=255), `0.5` exactly on the edge, higher inside.
pub fn coverage_to_sdf(width: u32, height: u32, coverage: &[u8], spread: f32) -> Image {
    let (w, h) = (width as usize, height as usize);
    assert_eq!(coverage.len(), w * h, "coverage length must be width*height");

    let inside = |i: usize| coverage[i] >= 128;
    let dist_to_inside = distance_transform(w, h, |i| inside(i));
    let dist_to_outside = distance_transform(w, h, |i| !inside(i));

    let spread = spread.max(1e-3);
    let mut img = Image::new(width, height);
    for i in 0..w * h {
        // Positive inside the glyph.
        let signed = dist_to_outside[i] - dist_to_inside[i];
        let n = (0.5 + signed / (2.0 * spread)).clamp(0.0, 1.0);
        let v = (n * 255.0).round() as u8;
        let x = (i % w) as u32;
        let y = (i / w) as u32;
        img.set_pixel(x, y, [v, v, v, 255]);
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A filled disk of radius `r` centred in a `size`×`size` mask.
    fn disk(size: usize, r: f32) -> (u32, Vec<u8>) {
        let c = size as f32 / 2.0;
        let mut cov = vec![0u8; size * size];
        for y in 0..size {
            for x in 0..size {
                let d = ((x as f32 + 0.5 - c).powi(2) + (y as f32 + 0.5 - c).powi(2)).sqrt();
                if d <= r {
                    cov[y * size + x] = 255;
                }
            }
        }
        (size as u32, cov)
    }

    #[test]
    fn center_is_inside_edges_are_outside() {
        let (size, cov) = disk(32, 10.0);
        let sdf = coverage_to_sdf(size, size, &cov, 8.0);
        let center = sdf.pixel(16, 16).unwrap()[0];
        let corner = sdf.pixel(0, 0).unwrap()[0];
        assert!(center > 160, "inside should be bright, got {center}");
        assert!(corner < 96, "outside should be dark, got {corner}");
    }

    #[test]
    fn edge_is_near_half() {
        let (size, cov) = disk(32, 10.0);
        let sdf = coverage_to_sdf(size, size, &cov, 8.0);
        // A pixel right at radius ~10 from centre (x=26, y=16) sits near the edge.
        let edge = sdf.pixel(26, 16).unwrap()[0] as i32;
        assert!((edge - 128).abs() < 40, "edge value {edge} should be near 128");
    }

    #[test]
    fn monotonic_outward() {
        let (size, cov) = disk(48, 16.0);
        let sdf = coverage_to_sdf(size, size, &cov, 12.0);
        // Moving from centre outward, the SDF value should not increase.
        let c = sdf.pixel(24, 24).unwrap()[0];
        let mid = sdf.pixel(34, 24).unwrap()[0];
        let out = sdf.pixel(44, 24).unwrap()[0];
        assert!(c >= mid && mid >= out);
    }

    #[test]
    #[should_panic]
    fn wrong_coverage_length_panics() {
        coverage_to_sdf(4, 4, &[0u8; 3], 4.0);
    }
}
