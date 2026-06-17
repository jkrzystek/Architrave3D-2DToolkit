use serde::{Deserialize, Serialize};

use crate::grid::Grid2D;

/// Configuration for the 2D fluid simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FluidConfig {
    pub viscosity: f32,
    pub diffusion: f32,
    pub dt: f32,
    pub gauss_seidel_iterations: u32,
}

impl Default for FluidConfig {
    fn default() -> Self {
        Self {
            viscosity: 0.0001,
            diffusion: 0.0001,
            dt: 0.1,
            gauss_seidel_iterations: 20,
        }
    }
}

/// 2D semi-Lagrangian fluid solver based on Jos Stam's "Stable Fluids" method.
pub struct FluidSim {
    width: usize,
    height: usize,
    config: FluidConfig,

    pub velocity_x: Grid2D<f32>,
    pub velocity_y: Grid2D<f32>,
    pub density: Grid2D<f32>,

    velocity_x_prev: Grid2D<f32>,
    velocity_y_prev: Grid2D<f32>,
    density_prev: Grid2D<f32>,
}

impl FluidSim {
    pub fn new(width: usize, height: usize, config: FluidConfig) -> Self {
        let zero = || Grid2D::new(width, height, 0.0_f32);
        Self {
            width,
            height,
            config,
            velocity_x: zero(),
            velocity_y: zero(),
            density: zero(),
            velocity_x_prev: zero(),
            velocity_y_prev: zero(),
            density_prev: zero(),
        }
    }

    /// Splat density at a grid position.
    pub fn add_density(&mut self, x: usize, y: usize, amount: f32) {
        if x < self.width && y < self.height {
            let cur = *self.density.get(x, y);
            self.density.set(x, y, cur + amount);
        }
    }

    /// Add velocity (force) at a grid position.
    pub fn add_velocity(&mut self, x: usize, y: usize, vx: f32, vy: f32) {
        if x < self.width && y < self.height {
            let cx = *self.velocity_x.get(x, y);
            let cy = *self.velocity_y.get(x, y);
            self.velocity_x.set(x, y, cx + vx);
            self.velocity_y.set(x, y, cy + vy);
        }
    }

    /// Perform one simulation step.
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let visc = self.config.viscosity;
        let diff = self.config.diffusion;
        let iters = self.config.gauss_seidel_iterations;
        let w = self.width;
        let h = self.height;

        // --- Velocity step ---

        // Diffuse velocity.
        self.velocity_x.swap(&mut self.velocity_x_prev);
        diffuse(&mut self.velocity_x, &self.velocity_x_prev, visc, dt, iters, w, h, 1);

        self.velocity_y.swap(&mut self.velocity_y_prev);
        diffuse(&mut self.velocity_y, &self.velocity_y_prev, visc, dt, iters, w, h, 2);

        // Project to make divergence-free.
        project(
            &mut self.velocity_x,
            &mut self.velocity_y,
            &mut self.velocity_x_prev, // reuse as pressure
            &mut self.velocity_y_prev, // reuse as divergence
            iters,
            w,
            h,
        );

        // Advect velocity.
        self.velocity_x.swap(&mut self.velocity_x_prev);
        self.velocity_y.swap(&mut self.velocity_y_prev);
        advect(
            &mut self.velocity_x,
            &self.velocity_x_prev,
            &self.velocity_x_prev,
            &self.velocity_y_prev,
            dt,
            w,
            h,
            1,
        );
        advect(
            &mut self.velocity_y,
            &self.velocity_y_prev,
            &self.velocity_x_prev,
            &self.velocity_y_prev,
            dt,
            w,
            h,
            2,
        );

        // Project again.
        project(
            &mut self.velocity_x,
            &mut self.velocity_y,
            &mut self.velocity_x_prev,
            &mut self.velocity_y_prev,
            iters,
            w,
            h,
        );

        // --- Density step ---

        self.density.swap(&mut self.density_prev);
        diffuse(&mut self.density, &self.density_prev, diff, dt, iters, w, h, 0);

        self.density.swap(&mut self.density_prev);
        advect(
            &mut self.density,
            &self.density_prev,
            &self.velocity_x,
            &self.velocity_y,
            dt,
            w,
            h,
            0,
        );
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn config(&self) -> &FluidConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: FluidConfig) {
        self.config = config;
    }

    pub fn density_grid(&self) -> &Grid2D<f32> {
        &self.density
    }

    pub fn velocity_at(&self, x: usize, y: usize) -> (f32, f32) {
        (*self.velocity_x.get(x, y), *self.velocity_y.get(x, y))
    }

    pub fn reset(&mut self) {
        self.velocity_x.fill(0.0);
        self.velocity_y.fill(0.0);
        self.density.fill(0.0);
        self.velocity_x_prev.fill(0.0);
        self.velocity_y_prev.fill(0.0);
        self.density_prev.fill(0.0);
    }
}

// ---------------------------------------------------------------------------
// Stable-Fluids helper functions
// ---------------------------------------------------------------------------

/// Gauss-Seidel diffusion step.
///
/// `boundary_type`: 0 = density (no negation), 1 = horizontal velocity
/// (negate at vertical walls), 2 = vertical velocity (negate at horizontal walls).
fn diffuse(
    grid: &mut Grid2D<f32>,
    prev: &Grid2D<f32>,
    diff: f32,
    dt: f32,
    iterations: u32,
    w: usize,
    h: usize,
    boundary_type: u8,
) {
    if w < 3 || h < 3 {
        return;
    }
    let a = dt * diff * ((w - 2) as f32) * ((h - 2) as f32);
    let denom = 1.0 + 4.0 * a;

    for _ in 0..iterations {
        for j in 1..h - 1 {
            for i in 1..w - 1 {
                let neighbors = *grid.get(i - 1, j)
                    + *grid.get(i + 1, j)
                    + *grid.get(i, j - 1)
                    + *grid.get(i, j + 1);
                let val = (*prev.get(i, j) + a * neighbors) / denom;
                grid.set(i, j, val);
            }
        }
        set_boundary(grid, boundary_type, w, h);
    }
}

/// Semi-Lagrangian advection: backtrace from each cell along velocity,
/// sample from `prev`.
fn advect(
    grid: &mut Grid2D<f32>,
    prev: &Grid2D<f32>,
    vel_x: &Grid2D<f32>,
    vel_y: &Grid2D<f32>,
    dt: f32,
    w: usize,
    h: usize,
    boundary_type: u8,
) {
    if w < 3 || h < 3 {
        return;
    }
    let dt_x = dt * (w - 2) as f32;
    let dt_y = dt * (h - 2) as f32;

    for j in 1..h - 1 {
        for i in 1..w - 1 {
            // Backtrace.
            let x = (i as f32) - dt_x * *vel_x.get(i, j);
            let y = (j as f32) - dt_y * *vel_y.get(i, j);

            // Clamp to interior.
            let x = x.clamp(0.5, (w as f32) - 1.5);
            let y = y.clamp(0.5, (h as f32) - 1.5);

            let val = prev.sample_bilinear(x, y);
            grid.set(i, j, val);
        }
    }
    set_boundary(grid, boundary_type, w, h);
}

/// Pressure projection to enforce incompressibility.
///
/// Uses `pressure` and `divergence` as scratch grids (same dimensions as velocity).
fn project(
    vel_x: &mut Grid2D<f32>,
    vel_y: &mut Grid2D<f32>,
    pressure: &mut Grid2D<f32>,
    divergence: &mut Grid2D<f32>,
    iterations: u32,
    w: usize,
    h: usize,
) {
    if w < 3 || h < 3 {
        return;
    }
    let inv_w = -0.5 / (w - 2) as f32;
    let inv_h = -0.5 / (h - 2) as f32;

    // Compute divergence.
    for j in 1..h - 1 {
        for i in 1..w - 1 {
            let div = inv_w * (*vel_x.get(i + 1, j) - *vel_x.get(i - 1, j))
                + inv_h * (*vel_y.get(i, j + 1) - *vel_y.get(i, j - 1));
            divergence.set(i, j, div);
            pressure.set(i, j, 0.0);
        }
    }
    set_boundary(divergence, 0, w, h);
    set_boundary(pressure, 0, w, h);

    // Solve pressure via Gauss-Seidel.
    for _ in 0..iterations {
        for j in 1..h - 1 {
            for i in 1..w - 1 {
                let p = (*divergence.get(i, j)
                    + *pressure.get(i - 1, j)
                    + *pressure.get(i + 1, j)
                    + *pressure.get(i, j - 1)
                    + *pressure.get(i, j + 1))
                    / 4.0;
                pressure.set(i, j, p);
            }
        }
        set_boundary(pressure, 0, w, h);
    }

    // Subtract pressure gradient from velocity.
    let half_w = 0.5 * (w - 2) as f32;
    let half_h = 0.5 * (h - 2) as f32;
    for j in 1..h - 1 {
        for i in 1..w - 1 {
            let vx = *vel_x.get(i, j)
                - half_w * (*pressure.get(i + 1, j) - *pressure.get(i - 1, j));
            let vy = *vel_y.get(i, j)
                - half_h * (*pressure.get(i, j + 1) - *pressure.get(i, j - 1));
            vel_x.set(i, j, vx);
            vel_y.set(i, j, vy);
        }
    }
    set_boundary(vel_x, 1, w, h);
    set_boundary(vel_y, 2, w, h);
}

/// Apply boundary conditions.
///
/// `boundary_type`: 0 = copy, 1 = negate x at vertical walls,
/// 2 = negate y at horizontal walls.
fn set_boundary(grid: &mut Grid2D<f32>, boundary_type: u8, w: usize, h: usize) {
    if w < 2 || h < 2 {
        return;
    }
    let sign_x: f32 = if boundary_type == 1 { -1.0 } else { 1.0 };
    let sign_y: f32 = if boundary_type == 2 { -1.0 } else { 1.0 };

    // Left and right walls.
    for j in 1..h - 1 {
        let left = *grid.get(1, j);
        grid.set(0, j, sign_x * left);
        let right = *grid.get(w - 2, j);
        grid.set(w - 1, j, sign_x * right);
    }

    // Top and bottom walls.
    for i in 1..w - 1 {
        let top = *grid.get(i, 1);
        grid.set(i, 0, sign_y * top);
        let bottom = *grid.get(i, h - 2);
        grid.set(i, h - 1, sign_y * bottom);
    }

    // Corners: average of two adjacent boundary cells.
    let c00 = 0.5 * (*grid.get(1, 0) + *grid.get(0, 1));
    grid.set(0, 0, c00);

    let c10 = 0.5 * (*grid.get(w - 2, 0) + *grid.get(w - 1, 1));
    grid.set(w - 1, 0, c10);

    let c01 = 0.5 * (*grid.get(1, h - 1) + *grid.get(0, h - 2));
    grid.set(0, h - 1, c01);

    let c11 = 0.5 * (*grid.get(w - 2, h - 1) + *grid.get(w - 1, h - 2));
    grid.set(w - 1, h - 1, c11);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total_density(sim: &FluidSim) -> f32 {
        sim.density_grid().sum()
    }

    #[test]
    fn density_conservation_approximately() {
        let config = FluidConfig {
            viscosity: 0.0,
            diffusion: 0.0001,
            dt: 0.05,
            gauss_seidel_iterations: 20,
        };
        let mut sim = FluidSim::new(32, 32, config);

        // Add a blob of density in the center.
        for dy in 14..18 {
            for dx in 14..18 {
                sim.add_density(dx, dy, 10.0);
            }
        }

        let total_before = total_density(&sim);
        assert!(total_before > 0.0);

        for _ in 0..10 {
            sim.step();
        }

        let total_after = total_density(&sim);
        // Density should be approximately conserved (within a reasonable margin
        // for the boundary treatment and numerical diffusion).
        let ratio = total_after / total_before;
        assert!(
            ratio > 0.7 && ratio < 1.3,
            "Density ratio {ratio} is too far from 1.0 (before={total_before}, after={total_after})"
        );
    }

    #[test]
    fn velocity_moves_density() {
        let config = FluidConfig {
            viscosity: 0.0,
            diffusion: 0.0,
            dt: 0.1,
            gauss_seidel_iterations: 20,
        };
        let mut sim = FluidSim::new(32, 32, config);

        // Place density on the left.
        sim.add_density(5, 16, 100.0);

        // Add rightward velocity.
        for y in 1..31 {
            for x in 1..31 {
                sim.add_velocity(x, y, 1.0, 0.0);
            }
        }

        let density_left_before = *sim.density_grid().get(5, 16);

        for _ in 0..20 {
            sim.step();
        }

        let density_left_after = *sim.density_grid().get(5, 16);

        // The density at the original position should have decreased
        // (some of it moved rightward).
        assert!(
            density_left_after < density_left_before,
            "Density should have moved: before={density_left_before}, after={density_left_after}"
        );
    }

    #[test]
    fn reset_clears_state() {
        let config = FluidConfig::default();
        let mut sim = FluidSim::new(16, 16, config);

        sim.add_density(8, 8, 50.0);
        sim.add_velocity(8, 8, 1.0, -1.0);
        sim.step();

        assert!(total_density(&sim) > 0.0);

        sim.reset();
        assert_eq!(total_density(&sim), 0.0);
        assert_eq!(sim.velocity_at(8, 8), (0.0, 0.0));
    }

    #[test]
    fn zero_input_stays_zero() {
        let config = FluidConfig::default();
        let mut sim = FluidSim::new(16, 16, config);

        for _ in 0..5 {
            sim.step();
        }

        // With no input, everything should remain zero.
        assert_eq!(total_density(&sim), 0.0);
    }
}
