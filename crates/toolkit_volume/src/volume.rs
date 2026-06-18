//! A dense regular 3D grid of values placed in world space.
//!
//! Values live at lattice points (sample corners), not cell centers, so a
//! `size` of `[nx, ny, nz]` stores `nx*ny*nz` samples. The grid is positioned by
//! an `origin` (world position of lattice point `(0,0,0)`) and an axis-aligned
//! `cell_size` (world distance between adjacent lattice points). This is the
//! substrate for 3D density/SDF fields, voxel sculpting, and 3D simulation.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::sample::VolumeSample;

/// A dense 3D grid of `T`, addressed by integer lattice coordinates and sampled
/// in continuous world space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Volume<T> {
    size: [usize; 3],
    origin: Vec3,
    cell_size: Vec3,
    data: Vec<T>,
}

impl<T: Clone> Volume<T> {
    /// Create a grid of `size` lattice points filled with `value`.
    pub fn new(size: [usize; 3], origin: Vec3, cell_size: Vec3, value: T) -> Self {
        let count = size[0] * size[1] * size[2];
        Self {
            size,
            origin,
            cell_size,
            data: vec![value; count],
        }
    }
}

impl<T> Volume<T> {
    /// Build a grid by evaluating `f` at each lattice coordinate `[x, y, z]`.
    pub fn from_fn(
        size: [usize; 3],
        origin: Vec3,
        cell_size: Vec3,
        mut f: impl FnMut([usize; 3]) -> T,
    ) -> Self {
        let mut data = Vec::with_capacity(size[0] * size[1] * size[2]);
        for z in 0..size[2] {
            for y in 0..size[1] {
                for x in 0..size[0] {
                    data.push(f([x, y, z]));
                }
            }
        }
        Self {
            size,
            origin,
            cell_size,
            data,
        }
    }

    /// Lattice dimensions `[nx, ny, nz]`.
    pub fn size(&self) -> [usize; 3] {
        self.size
    }

    /// World position of lattice point `(0,0,0)`.
    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    /// World distance between adjacent lattice points along each axis.
    pub fn cell_size(&self) -> Vec3 {
        self.cell_size
    }

    /// Total number of samples.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the grid has no samples.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Raw sample slice in `x`-fastest, then `y`, then `z` order.
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Mutable raw sample slice.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Whether `(x, y, z)` is a valid lattice coordinate.
    pub fn in_bounds(&self, x: usize, y: usize, z: usize) -> bool {
        x < self.size[0] && y < self.size[1] && z < self.size[2]
    }

    #[inline]
    fn linear(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.size[1] + y) * self.size[0] + x
    }

    /// Sample at lattice coordinate, or `None` if out of bounds.
    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<&T> {
        if self.in_bounds(x, y, z) {
            Some(&self.data[self.linear(x, y, z)])
        } else {
            None
        }
    }

    /// Mutable sample at lattice coordinate, or `None` if out of bounds.
    pub fn get_mut(&mut self, x: usize, y: usize, z: usize) -> Option<&mut T> {
        if self.in_bounds(x, y, z) {
            let i = self.linear(x, y, z);
            Some(&mut self.data[i])
        } else {
            None
        }
    }

    /// Set the sample at a lattice coordinate; ignored if out of bounds.
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: T) {
        if self.in_bounds(x, y, z) {
            let i = self.linear(x, y, z);
            self.data[i] = value;
        }
    }

    /// World position of lattice point `(x, y, z)`.
    pub fn world_position(&self, x: usize, y: usize, z: usize) -> Vec3 {
        self.origin + self.cell_size * Vec3::new(x as f32, y as f32, z as f32)
    }

    /// Continuous lattice coordinates of a world point (may be fractional /
    /// outside the grid).
    pub fn world_to_grid(&self, p: Vec3) -> Vec3 {
        (p - self.origin) / self.cell_size
    }

    /// World-space minimum corner (origin).
    pub fn min_corner(&self) -> Vec3 {
        self.origin
    }

    /// World-space maximum corner (last lattice point).
    pub fn max_corner(&self) -> Vec3 {
        self.world_position(
            self.size[0].saturating_sub(1),
            self.size[1].saturating_sub(1),
            self.size[2].saturating_sub(1),
        )
    }
}

impl<T: VolumeSample> Volume<T> {
    /// Sample with edge clamping: out-of-range coordinates read the nearest
    /// in-range lattice point (so sampling at the border is well defined).
    pub fn get_clamped(&self, x: i64, y: i64, z: i64) -> T {
        if self.size[0] == 0 {
            return T::zero();
        }
        let cx = x.clamp(0, self.size[0] as i64 - 1) as usize;
        let cy = y.clamp(0, self.size[1] as i64 - 1) as usize;
        let cz = z.clamp(0, self.size[2] as i64 - 1) as usize;
        self.data[self.linear(cx, cy, cz)]
    }

    /// Trilinearly sample the field at a world point, clamping at the borders.
    pub fn sample(&self, world: Vec3) -> T {
        let g = self.world_to_grid(world);
        let x0 = g.x.floor();
        let y0 = g.y.floor();
        let z0 = g.z.floor();
        let fx = g.x - x0;
        let fy = g.y - y0;
        let fz = g.z - z0;
        let (xi, yi, zi) = (x0 as i64, y0 as i64, z0 as i64);

        let c000 = self.get_clamped(xi, yi, zi);
        let c100 = self.get_clamped(xi + 1, yi, zi);
        let c010 = self.get_clamped(xi, yi + 1, zi);
        let c110 = self.get_clamped(xi + 1, yi + 1, zi);
        let c001 = self.get_clamped(xi, yi, zi + 1);
        let c101 = self.get_clamped(xi + 1, yi, zi + 1);
        let c011 = self.get_clamped(xi, yi + 1, zi + 1);
        let c111 = self.get_clamped(xi + 1, yi + 1, zi + 1);

        let x00 = c000.lerp(c100, fx);
        let x10 = c010.lerp(c110, fx);
        let x01 = c001.lerp(c101, fx);
        let x11 = c011.lerp(c111, fx);
        let y0v = x00.lerp(x10, fy);
        let y1v = x01.lerp(x11, fy);
        y0v.lerp(y1v, fz)
    }

    /// Resample this volume onto a new lattice of `new_size` covering the same
    /// world bounds, using trilinear interpolation.
    pub fn resample(&self, new_size: [usize; 3]) -> Volume<T> {
        let span = self.max_corner() - self.min_corner();
        let cell = Vec3::new(
            if new_size[0] > 1 { span.x / (new_size[0] - 1) as f32 } else { 0.0 },
            if new_size[1] > 1 { span.y / (new_size[1] - 1) as f32 } else { 0.0 },
            if new_size[2] > 1 { span.z / (new_size[2] - 1) as f32 } else { 0.0 },
        );
        let origin = self.origin;
        Volume::from_fn(new_size, origin, cell, |[x, y, z]| {
            let p = origin + cell * Vec3::new(x as f32, y as f32, z as f32);
            self.sample(p)
        })
    }
}

impl Volume<f32> {
    /// Central-difference gradient of a scalar field at a world point, using the
    /// cell size as the step. Returns the spatial derivative `(d/dx, d/dy, d/dz)`.
    pub fn gradient(&self, world: Vec3) -> Vec3 {
        let h = self.cell_size;
        let dx = (self.sample(world + Vec3::new(h.x, 0.0, 0.0))
            - self.sample(world - Vec3::new(h.x, 0.0, 0.0)))
            / (2.0 * h.x);
        let dy = (self.sample(world + Vec3::new(0.0, h.y, 0.0))
            - self.sample(world - Vec3::new(0.0, h.y, 0.0)))
            / (2.0 * h.y);
        let dz = (self.sample(world + Vec3::new(0.0, 0.0, h.z))
            - self.sample(world - Vec3::new(0.0, 0.0, h.z)))
            / (2.0 * h.z);
        Vec3::new(dx, dy, dz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexing_and_bounds() {
        let mut v = Volume::new([2, 3, 4], Vec3::ZERO, Vec3::ONE, 0.0_f32);
        assert_eq!(v.len(), 24);
        v.set(1, 2, 3, 7.0);
        assert_eq!(v.get(1, 2, 3), Some(&7.0));
        assert!(v.get(2, 0, 0).is_none());
        assert!(!v.in_bounds(2, 0, 0));
    }

    #[test]
    fn world_position_uses_origin_and_cell() {
        let v = Volume::new([4, 4, 4], Vec3::new(1.0, 2.0, 3.0), Vec3::splat(0.5), 0.0_f32);
        assert_eq!(v.world_position(2, 0, 0), Vec3::new(2.0, 2.0, 3.0));
        assert_eq!(v.max_corner(), Vec3::new(1.0 + 1.5, 2.0 + 1.5, 3.0 + 1.5));
    }

    #[test]
    fn trilinear_sample_midpoint() {
        // Linear ramp along x: value == x index.
        let v = Volume::from_fn([2, 1, 1], Vec3::ZERO, Vec3::ONE, |[x, _, _]| x as f32);
        // Halfway between lattice 0 and 1 in world space (x = 0.5).
        assert!((v.sample(Vec3::new(0.5, 0.0, 0.0)) - 0.5).abs() < 1e-6);
        // Clamps beyond the border.
        assert!((v.sample(Vec3::new(5.0, 0.0, 0.0)) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn gradient_of_linear_ramp() {
        // f(x) = 2x  -> gradient (2, 0, 0).
        let v = Volume::from_fn([5, 5, 5], Vec3::ZERO, Vec3::ONE, |[x, _, _]| 2.0 * x as f32);
        let g = v.gradient(Vec3::new(2.0, 2.0, 2.0));
        assert!((g.x - 2.0).abs() < 1e-4, "gx = {}", g.x);
        assert!(g.y.abs() < 1e-4 && g.z.abs() < 1e-4);
    }

    #[test]
    fn resample_preserves_ramp() {
        let v = Volume::from_fn([3, 1, 1], Vec3::ZERO, Vec3::ONE, |[x, _, _]| x as f32);
        let r = v.resample([5, 1, 1]);
        assert_eq!(r.size(), [5, 1, 1]);
        // Endpoints preserved; midpoint interpolated.
        assert!((r.get_clamped(0, 0, 0) - 0.0).abs() < 1e-6);
        assert!((r.get_clamped(4, 0, 0) - 2.0).abs() < 1e-6);
        assert!((r.get_clamped(2, 0, 0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn vector_volume_samples() {
        let v = Volume::new([2, 2, 2], Vec3::ZERO, Vec3::ONE, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(v.sample(Vec3::splat(0.5)), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn serde_roundtrip() {
        let v = Volume::new([2, 2, 2], Vec3::ZERO, Vec3::ONE, 1.5_f32);
        let json = serde_json::to_string(&v).unwrap();
        let back: Volume<f32> = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }
}
