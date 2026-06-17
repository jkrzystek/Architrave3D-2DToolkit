//! Geometric sampling built on [`Rng`]: uniform points on/in disks and spheres,
//! and Poisson-disk (blue-noise) point sets.

use glam::{Vec2, Vec3};
use std::f32::consts::TAU;

use crate::rng::Rng;

impl Rng {
    /// Uniform point on the unit circle.
    pub fn unit_vec2(&mut self) -> Vec2 {
        let a = self.range_f32(0.0, TAU);
        Vec2::new(a.cos(), a.sin())
    }

    /// Uniform point inside the unit disk.
    pub fn in_unit_disk(&mut self) -> Vec2 {
        // r = sqrt(u) gives a uniform area distribution.
        let r = self.next_f32().sqrt();
        let a = self.range_f32(0.0, TAU);
        Vec2::new(r * a.cos(), r * a.sin())
    }

    /// Uniform direction on the unit sphere.
    pub fn unit_vec3(&mut self) -> Vec3 {
        let z = self.range_f32(-1.0, 1.0);
        let a = self.range_f32(0.0, TAU);
        let r = (1.0 - z * z).max(0.0).sqrt();
        Vec3::new(r * a.cos(), r * a.sin(), z)
    }

    /// Uniform point inside the unit sphere.
    pub fn in_unit_sphere(&mut self) -> Vec3 {
        // radius scaled by cube root for uniform volume density.
        let r = self.next_f32().cbrt();
        self.unit_vec3() * r
    }

    /// Uniform direction on the hemisphere around `normal`.
    pub fn on_hemisphere(&mut self, normal: Vec3) -> Vec3 {
        let v = self.unit_vec3();
        if v.dot(normal) < 0.0 {
            -v
        } else {
            v
        }
    }
}

/// Generate a 2D Poisson-disk (blue-noise) point set in `[0,width] × [0,height]`
/// with no two points closer than `radius`. Uses Bridson's algorithm.
///
/// `k` is the number of candidate samples per active point (30 is typical).
pub fn poisson_disk_2d(
    rng: &mut Rng,
    width: f32,
    height: f32,
    radius: f32,
    k: usize,
) -> Vec<Vec2> {
    let radius = radius.max(1e-4);
    let cell = radius / (2.0_f32).sqrt();
    let gw = (width / cell).ceil() as usize + 1;
    let gh = (height / cell).ceil() as usize + 1;
    let mut grid: Vec<i32> = vec![-1; gw * gh];
    let mut points: Vec<Vec2> = Vec::new();
    let mut active: Vec<usize> = Vec::new();

    let grid_index = |p: Vec2| -> usize {
        let gx = (p.x / cell) as usize;
        let gy = (p.y / cell) as usize;
        gy.min(gh - 1) * gw + gx.min(gw - 1)
    };

    // Initial point at the center.
    let first = Vec2::new(width * 0.5, height * 0.5);
    grid[grid_index(first)] = 0;
    points.push(first);
    active.push(0);

    while !active.is_empty() {
        let ai = rng.range_u32(0, active.len() as u32) as usize;
        let center = points[active[ai]];
        let mut found = false;

        for _ in 0..k {
            // Candidate in the annulus [radius, 2*radius].
            let ang = rng.range_f32(0.0, TAU);
            let dist = rng.range_f32(radius, 2.0 * radius);
            let cand = center + Vec2::new(ang.cos(), ang.sin()) * dist;

            if cand.x < 0.0 || cand.y < 0.0 || cand.x >= width || cand.y >= height {
                continue;
            }
            if is_far_enough(cand, radius, cell, gw, gh, &grid, &points) {
                let idx = points.len();
                grid[grid_index(cand)] = idx as i32;
                points.push(cand);
                active.push(idx);
                found = true;
                break;
            }
        }
        if !found {
            active.swap_remove(ai);
        }
    }
    points
}

fn is_far_enough(
    cand: Vec2,
    radius: f32,
    cell: f32,
    gw: usize,
    gh: usize,
    grid: &[i32],
    points: &[Vec2],
) -> bool {
    let gx = (cand.x / cell) as isize;
    let gy = (cand.y / cell) as isize;
    let r2 = radius * radius;
    for dy in -2..=2 {
        for dx in -2..=2 {
            let nx = gx + dx;
            let ny = gy + dy;
            if nx < 0 || ny < 0 || nx as usize >= gw || ny as usize >= gh {
                continue;
            }
            let stored = grid[ny as usize * gw + nx as usize];
            if stored >= 0 && points[stored as usize].distance_squared(cand) < r2 {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_vec3_is_unit_length() {
        let mut r = Rng::seed_from_u64(1);
        for _ in 0..1000 {
            assert!((r.unit_vec3().length() - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn in_unit_disk_within_radius() {
        let mut r = Rng::seed_from_u64(2);
        for _ in 0..1000 {
            assert!(r.in_unit_disk().length() <= 1.0 + 1e-5);
        }
    }

    #[test]
    fn in_unit_sphere_within_radius() {
        let mut r = Rng::seed_from_u64(3);
        for _ in 0..1000 {
            assert!(r.in_unit_sphere().length() <= 1.0 + 1e-5);
        }
    }

    #[test]
    fn hemisphere_faces_normal() {
        let mut r = Rng::seed_from_u64(4);
        let n = Vec3::Y;
        for _ in 0..1000 {
            assert!(r.on_hemisphere(n).dot(n) >= 0.0);
        }
    }

    #[test]
    fn poisson_respects_min_distance() {
        let mut r = Rng::seed_from_u64(5);
        let radius = 5.0;
        let pts = poisson_disk_2d(&mut r, 100.0, 100.0, radius, 30);
        assert!(pts.len() > 10, "expected a decent number of points");
        for i in 0..pts.len() {
            assert!(pts[i].x >= 0.0 && pts[i].x < 100.0);
            for j in i + 1..pts.len() {
                assert!(
                    pts[i].distance(pts[j]) >= radius - 1e-3,
                    "points too close"
                );
            }
        }
    }
}
