use serde::{Deserialize, Serialize};

/// A 2D grid of cloneable values stored in row-major order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid2D<T: Clone> {
    width: usize,
    height: usize,
    data: Vec<T>,
}

impl<T: Clone> Grid2D<T> {
    /// Create a new grid filled with `default`.
    pub fn new(width: usize, height: usize, default: T) -> Self {
        Self {
            width,
            height,
            data: vec![default; width * height],
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Get a reference to the cell at `(x, y)`.
    /// Returns `None` if out of bounds.
    #[inline]
    pub fn try_get(&self, x: usize, y: usize) -> Option<&T> {
        if x < self.width && y < self.height {
            Some(&self.data[y * self.width + x])
        } else {
            None
        }
    }

    /// Get a reference, clamping coordinates to valid range.
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> &T {
        let cx = x.min(self.width.saturating_sub(1));
        let cy = y.min(self.height.saturating_sub(1));
        &self.data[cy * self.width + cx]
    }

    /// Get a mutable reference, clamping coordinates.
    #[inline]
    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut T {
        let cx = x.min(self.width.saturating_sub(1));
        let cy = y.min(self.height.saturating_sub(1));
        &mut self.data[cy * self.width + cx]
    }

    /// Set a cell value. Out-of-bounds writes are silently ignored.
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, value: T) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = value;
        }
    }

    /// Swap the contents of two same-sized grids.
    /// Panics in debug mode if dimensions differ.
    pub fn swap(&mut self, other: &mut Grid2D<T>) {
        debug_assert_eq!(self.width, other.width);
        debug_assert_eq!(self.height, other.height);
        std::mem::swap(&mut self.data, &mut other.data);
    }

    /// Fill every cell with `value`.
    pub fn fill(&mut self, value: T) {
        self.data.fill(value);
    }

    #[inline]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }
}

// ---------------------------------------------------------------------------
// f32-specific methods
// ---------------------------------------------------------------------------

impl Grid2D<f32> {
    /// Bilinear interpolation at fractional coordinates.
    /// Coordinates are clamped to the grid extent.
    pub fn sample_bilinear(&self, x: f32, y: f32) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 0.0;
        }

        let max_x = (self.width as f32) - 1.0;
        let max_y = (self.height as f32) - 1.0;

        let x = x.clamp(0.0, max_x);
        let y = y.clamp(0.0, max_y);

        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let fx = x - x.floor();
        let fy = y - y.floor();

        let c00 = self.data[y0 * self.width + x0];
        let c10 = self.data[y0 * self.width + x1];
        let c01 = self.data[y1 * self.width + x0];
        let c11 = self.data[y1 * self.width + x1];

        let top = c00 * (1.0 - fx) + c10 * fx;
        let bot = c01 * (1.0 - fx) + c11 * fx;
        top * (1.0 - fy) + bot * fy
    }

    pub fn min(&self) -> f32 {
        self.data
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
    }

    pub fn max(&self) -> f32 {
        self.data
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn sum(&self) -> f32 {
        self.data.iter().sum()
    }

    /// `self[i] += other[i] * scale` for each cell.
    /// Dimensions must match.
    pub fn add_scaled(&mut self, other: &Grid2D<f32>, scale: f32) {
        debug_assert_eq!(self.width, other.width);
        debug_assert_eq!(self.height, other.height);
        for (a, b) in self.data.iter_mut().zip(other.data.iter()) {
            *a += *b * scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_grid_filled() {
        let g = Grid2D::new(4, 3, 1.0_f32);
        assert_eq!(g.width(), 4);
        assert_eq!(g.height(), 3);
        assert_eq!(g.data().len(), 12);
        assert!(g.data().iter().all(|&v| v == 1.0));
    }

    #[test]
    fn get_set_basic() {
        let mut g = Grid2D::new(5, 5, 0.0_f32);
        g.set(2, 3, 42.0);
        assert_eq!(*g.get(2, 3), 42.0);
        assert_eq!(*g.get(0, 0), 0.0);
    }

    #[test]
    fn out_of_bounds_get_clamps() {
        let g = Grid2D::new(3, 3, 7.0_f32);
        // Should clamp to edge values rather than panic.
        assert_eq!(*g.get(100, 100), 7.0);
        assert_eq!(*g.get(0, 100), 7.0);
    }

    #[test]
    fn out_of_bounds_set_ignored() {
        let mut g = Grid2D::new(3, 3, 0.0_f32);
        g.set(100, 100, 99.0); // should not panic
        assert_eq!(g.sum(), 0.0);
    }

    #[test]
    fn bilinear_at_integer_coords() {
        let mut g = Grid2D::new(3, 3, 0.0_f32);
        g.set(1, 1, 4.0);
        assert_eq!(g.sample_bilinear(1.0, 1.0), 4.0);
    }

    #[test]
    fn bilinear_midpoint() {
        // 2x2 grid:
        // [0, 2]
        // [0, 2]
        let mut g = Grid2D::new(2, 2, 0.0_f32);
        g.set(1, 0, 2.0);
        g.set(1, 1, 2.0);
        // At x=0.5, y=0.0: lerp between 0 and 2 = 1.0
        let v = g.sample_bilinear(0.5, 0.0);
        assert!((v - 1.0).abs() < 1e-6, "got {v}");
    }

    #[test]
    fn bilinear_center_of_four() {
        // 2x2 grid:
        // [1, 3]
        // [5, 7]
        let mut g = Grid2D::new(2, 2, 0.0_f32);
        g.set(0, 0, 1.0);
        g.set(1, 0, 3.0);
        g.set(0, 1, 5.0);
        g.set(1, 1, 7.0);
        // Center (0.5, 0.5): average of 1,3,5,7 = 4.0
        let v = g.sample_bilinear(0.5, 0.5);
        assert!((v - 4.0).abs() < 1e-6, "got {v}");
    }

    #[test]
    fn bilinear_clamps_negative() {
        let g = Grid2D::new(2, 2, 5.0_f32);
        let v = g.sample_bilinear(-1.0, -1.0);
        assert_eq!(v, 5.0);
    }

    #[test]
    fn min_max_sum() {
        let mut g = Grid2D::new(3, 1, 0.0_f32);
        g.set(0, 0, -5.0);
        g.set(1, 0, 3.0);
        g.set(2, 0, 10.0);
        assert_eq!(g.min(), -5.0);
        assert_eq!(g.max(), 10.0);
        assert!((g.sum() - 8.0).abs() < 1e-6);
    }

    #[test]
    fn add_scaled() {
        let mut a = Grid2D::new(2, 2, 1.0_f32);
        let b = Grid2D::new(2, 2, 3.0_f32);
        a.add_scaled(&b, 2.0);
        // Each cell: 1 + 3*2 = 7
        assert!(a.data().iter().all(|&v| (v - 7.0).abs() < 1e-6));
    }

    #[test]
    fn swap_grids() {
        let mut a = Grid2D::new(2, 2, 1.0_f32);
        let mut b = Grid2D::new(2, 2, 9.0_f32);
        a.swap(&mut b);
        assert!(a.data().iter().all(|&v| v == 9.0));
        assert!(b.data().iter().all(|&v| v == 1.0));
    }

    #[test]
    fn fill_grid() {
        let mut g = Grid2D::new(3, 3, 0.0_f32);
        g.fill(42.0);
        assert!(g.data().iter().all(|&v| v == 42.0));
    }

    #[test]
    fn try_get_returns_none_oob() {
        let g = Grid2D::new(2, 2, 0.0_f32);
        assert!(g.try_get(0, 0).is_some());
        assert!(g.try_get(2, 0).is_none());
        assert!(g.try_get(0, 2).is_none());
    }
}
