use serde::{Deserialize, Serialize};

use crate::grid::Grid2D;

/// Configuration for hydraulic erosion simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErosionConfig {
    /// Amount of water added per cell per step (rain).
    pub rain_rate: f32,
    /// Fraction of water removed per step.
    pub evaporation_rate: f32,
    /// Maximum sediment a unit of water moving at unit speed can carry.
    pub sediment_capacity: f32,
    /// Rate at which excess sediment is deposited.
    pub deposition_rate: f32,
    /// Rate at which terrain is eroded when under capacity.
    pub erosion_rate: f32,
    /// Minimum slope used in capacity calculation to avoid division issues.
    pub min_slope: f32,
    /// Gravitational acceleration constant.
    pub gravity: f32,
    /// Default number of iterations for `run`.
    pub iterations: u32,
}

impl Default for ErosionConfig {
    fn default() -> Self {
        Self {
            rain_rate: 0.01,
            evaporation_rate: 0.02,
            sediment_capacity: 0.05,
            deposition_rate: 0.3,
            erosion_rate: 0.3,
            min_slope: 0.01,
            gravity: 9.81,
            iterations: 50,
        }
    }
}

/// Hydraulic erosion simulation on a heightfield.
pub struct ErosionSim {
    config: ErosionConfig,
    heightmap: Grid2D<f32>,
    water: Grid2D<f32>,
    sediment: Grid2D<f32>,
    velocity_x: Grid2D<f32>,
    velocity_y: Grid2D<f32>,
}

impl ErosionSim {
    pub fn new(heightmap: Grid2D<f32>, config: ErosionConfig) -> Self {
        let w = heightmap.width();
        let h = heightmap.height();
        Self {
            config,
            heightmap,
            water: Grid2D::new(w, h, 0.0),
            sediment: Grid2D::new(w, h, 0.0),
            velocity_x: Grid2D::new(w, h, 0.0),
            velocity_y: Grid2D::new(w, h, 0.0),
        }
    }

    /// Perform one erosion iteration.
    pub fn step(&mut self) {
        let w = self.heightmap.width();
        let h = self.heightmap.height();
        if w < 2 || h < 2 {
            return;
        }

        // 1. Add water (rain).
        for y in 0..h {
            for x in 0..w {
                let cur = *self.water.get(x, y);
                self.water.set(x, y, cur + self.config.rain_rate);
            }
        }

        // 2-3. Calculate flow based on height + water differences, update velocity.
        let mut new_vx = Grid2D::new(w, h, 0.0_f32);
        let mut new_vy = Grid2D::new(w, h, 0.0_f32);

        for y in 0..h {
            for x in 0..w {
                let here = *self.heightmap.get(x, y) + *self.water.get(x, y);

                // Compute gradient via neighbor differences.
                let mut gx = 0.0_f32;
                let mut gy = 0.0_f32;

                if x > 0 && x < w - 1 {
                    let left = *self.heightmap.get(x - 1, y) + *self.water.get(x - 1, y);
                    let right = *self.heightmap.get(x + 1, y) + *self.water.get(x + 1, y);
                    gx = (left - right) * 0.5;
                } else if x == 0 && w > 1 {
                    let right = *self.heightmap.get(x + 1, y) + *self.water.get(x + 1, y);
                    gx = here - right;
                } else if x == w - 1 && w > 1 {
                    let left = *self.heightmap.get(x - 1, y) + *self.water.get(x - 1, y);
                    gx = left - here;
                }

                if y > 0 && y < h - 1 {
                    let up = *self.heightmap.get(x, y - 1) + *self.water.get(x, y - 1);
                    let down = *self.heightmap.get(x, y + 1) + *self.water.get(x, y + 1);
                    gy = (up - down) * 0.5;
                } else if y == 0 && h > 1 {
                    let down = *self.heightmap.get(x, y + 1) + *self.water.get(x, y + 1);
                    gy = here - down;
                } else if y == h - 1 && h > 1 {
                    let up = *self.heightmap.get(x, y - 1) + *self.water.get(x, y - 1);
                    gy = up - here;
                }

                // Update velocity with gravity-driven acceleration and damping.
                let vx = *self.velocity_x.get(x, y) * 0.5 + gx * self.config.gravity;
                let vy = *self.velocity_y.get(x, y) * 0.5 + gy * self.config.gravity;

                new_vx.set(x, y, vx);
                new_vy.set(x, y, vy);
            }
        }

        self.velocity_x = new_vx;
        self.velocity_y = new_vy;

        // 4. Erode or deposit sediment based on carrying capacity.
        for y in 0..h {
            for x in 0..w {
                let vx = *self.velocity_x.get(x, y);
                let vy = *self.velocity_y.get(x, y);
                let speed = (vx * vx + vy * vy).sqrt();

                // Local slope approximation.
                let slope = self.local_slope(x, y).max(self.config.min_slope);

                let water_amount = *self.water.get(x, y);
                let capacity =
                    self.config.sediment_capacity * speed * slope * water_amount.max(0.001);

                let current_sediment = *self.sediment.get(x, y);
                let current_height = *self.heightmap.get(x, y);

                if current_sediment > capacity {
                    // Deposit excess.
                    let deposit = (current_sediment - capacity) * self.config.deposition_rate;
                    self.sediment.set(x, y, current_sediment - deposit);
                    self.heightmap.set(x, y, current_height + deposit);
                } else {
                    // Erode terrain.
                    let erode = (capacity - current_sediment) * self.config.erosion_rate;
                    // Don't erode more than available terrain (keep height >= 0).
                    let erode = erode.min(current_height.max(0.0));
                    self.sediment.set(x, y, current_sediment + erode);
                    self.heightmap.set(x, y, current_height - erode);
                }
            }
        }

        // 5. Transport sediment using velocity (semi-Lagrangian backtrace).
        let prev_sediment = self.sediment.clone();
        for y in 0..h {
            for x in 0..w {
                let vx = *self.velocity_x.get(x, y);
                let vy = *self.velocity_y.get(x, y);

                // Backtrace: where did this cell's sediment come from?
                let src_x = (x as f32) - vx;
                let src_y = (y as f32) - vy;

                let val = prev_sediment.sample_bilinear(src_x, src_y);
                self.sediment.set(x, y, val);
            }
        }

        // 6. Evaporate water.
        for y in 0..h {
            for x in 0..w {
                let cur = *self.water.get(x, y);
                let new_water = (cur * (1.0 - self.config.evaporation_rate)).max(0.0);
                self.water.set(x, y, new_water);
            }
        }
    }

    pub fn width(&self) -> usize {
        self.heightmap.width()
    }

    pub fn height(&self) -> usize {
        self.heightmap.height()
    }

    pub fn config(&self) -> &ErosionConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: ErosionConfig) {
        self.config = config;
    }

    pub fn heightmap(&self) -> &Grid2D<f32> {
        &self.heightmap
    }

    pub fn terrain(&self) -> &Grid2D<f32> {
        &self.heightmap
    }

    pub fn water(&self) -> &Grid2D<f32> {
        &self.water
    }

    pub fn water_map(&self) -> &Grid2D<f32> {
        &self.water
    }

    pub fn sediment(&self) -> &Grid2D<f32> {
        &self.sediment
    }

    /// Run multiple erosion steps.
    pub fn run(&mut self, iterations: u32) {
        for _ in 0..iterations {
            self.step();
        }
    }

    /// Reset with a new heightmap, clearing water and sediment.
    pub fn reset(&mut self, heightmap: Grid2D<f32>) {
        let w = heightmap.width();
        let h = heightmap.height();
        self.heightmap = heightmap;
        self.water = Grid2D::new(w, h, 0.0);
        self.sediment = Grid2D::new(w, h, 0.0);
        self.velocity_x = Grid2D::new(w, h, 0.0);
        self.velocity_y = Grid2D::new(w, h, 0.0);
    }

    // --- private helpers ---

    /// Compute local slope magnitude from height differences with neighbors.
    fn local_slope(&self, x: usize, y: usize) -> f32 {
        let w = self.heightmap.width();
        let h = self.heightmap.height();
        let center = *self.heightmap.get(x, y);

        let mut max_diff = 0.0_f32;

        if x > 0 {
            max_diff = max_diff.max((center - *self.heightmap.get(x - 1, y)).abs());
        }
        if x + 1 < w {
            max_diff = max_diff.max((center - *self.heightmap.get(x + 1, y)).abs());
        }
        if y > 0 {
            max_diff = max_diff.max((center - *self.heightmap.get(x, y - 1)).abs());
        }
        if y + 1 < h {
            max_diff = max_diff.max((center - *self.heightmap.get(x, y + 1)).abs());
        }

        max_diff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_terrain_stays_approximately_flat() {
        let heightmap = Grid2D::new(16, 16, 10.0_f32);
        let config = ErosionConfig {
            rain_rate: 0.01,
            evaporation_rate: 0.05,
            sediment_capacity: 0.05,
            deposition_rate: 0.3,
            erosion_rate: 0.3,
            min_slope: 0.01,
            gravity: 9.81,
            iterations: 20,
        };
        let mut sim = ErosionSim::new(heightmap, config);

        sim.run(20);

        // On a perfectly flat terrain the slope is near zero everywhere, so
        // the carrying capacity is tiny and changes should be minimal.
        let hm = sim.heightmap();
        let min_h = hm.min();
        let max_h = hm.max();
        let range = max_h - min_h;
        assert!(
            range < 1.0,
            "Flat terrain variance too large: min={min_h}, max={max_h}, range={range}"
        );
    }

    #[test]
    fn sloped_terrain_erodes_at_top() {
        // Create a terrain that slopes from 100 at y=0 to 0 at y=height-1.
        let size = 32;
        let mut heightmap = Grid2D::new(size, size, 0.0_f32);
        for y in 0..size {
            let h = 100.0 * (1.0 - y as f32 / (size - 1) as f32);
            for x in 0..size {
                heightmap.set(x, y, h);
            }
        }

        let top_center_before = *heightmap.get(size / 2, 1);

        let config = ErosionConfig {
            rain_rate: 0.02,
            evaporation_rate: 0.01,
            sediment_capacity: 0.1,
            deposition_rate: 0.2,
            erosion_rate: 0.5,
            min_slope: 0.01,
            gravity: 9.81,
            iterations: 100,
        };
        let mut sim = ErosionSim::new(heightmap, config);

        sim.run(100);

        let top_center_after = *sim.heightmap().get(size / 2, 1);
        assert!(
            top_center_after < top_center_before,
            "Top of slope should erode: before={top_center_before}, after={top_center_after}"
        );
    }

    #[test]
    fn water_evaporates_to_near_zero() {
        let heightmap = Grid2D::new(8, 8, 5.0_f32);
        let config = ErosionConfig {
            rain_rate: 0.0, // no rain
            evaporation_rate: 0.5,
            sediment_capacity: 0.05,
            deposition_rate: 0.3,
            erosion_rate: 0.3,
            min_slope: 0.01,
            gravity: 9.81,
            iterations: 50,
        };
        let mut sim = ErosionSim::new(heightmap, config);

        // Manually add some water.
        for y in 0..8 {
            for x in 0..8 {
                sim.water.set(x, y, 10.0);
            }
        }

        let water_before = sim.water_map().sum();
        assert!(water_before > 0.0);

        // Run many steps with no rain and high evaporation.
        sim.run(50);

        let water_after = sim.water_map().sum();
        assert!(
            water_after < 0.01,
            "Water should evaporate to near zero: {water_after}"
        );
    }

    #[test]
    fn reset_clears_simulation() {
        let heightmap = Grid2D::new(8, 8, 5.0_f32);
        let config = ErosionConfig::default();
        let mut sim = ErosionSim::new(heightmap, config);

        sim.run(10);

        let new_hm = Grid2D::new(8, 8, 20.0_f32);
        sim.reset(new_hm);

        assert_eq!(sim.water_map().sum(), 0.0);
        assert!((sim.heightmap().min() - 20.0).abs() < 1e-6);
        assert!((sim.heightmap().max() - 20.0).abs() < 1e-6);
    }
}
