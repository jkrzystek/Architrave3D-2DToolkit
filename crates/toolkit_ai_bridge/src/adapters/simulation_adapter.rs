use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{json, Value};
use toolkit_simulation::{FluidSim, ErosionSim};

use crate::error::{BridgeError, BridgeResult};
use crate::provider::*;

/// AI bridge for the 2D fluid simulation.
///
/// Exposes configuration, aggregate stats, and point sampling — NOT full grid dumps.
pub struct FluidBridge {
    sim: Arc<RwLock<FluidSim>>,
}

impl FluidBridge {
    pub fn new(sim: Arc<RwLock<FluidSim>>) -> Self {
        Self { sim }
    }
}

impl AiProvider for FluidBridge {
    fn namespace(&self) -> &str {
        "fluid"
    }

    fn description(&self) -> &str {
        "2D fluid simulation control. Configure viscosity/diffusion, step the sim, \
         and sample values at specific points. Full grid data is not exposed."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![ResourceDescriptor::json(
            "fluid://status",
            "Fluid Sim Status",
            "Grid dimensions, config, and aggregate stats (max velocity, total density)",
        )]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "fluid.step",
                "Advance the fluid simulation by one step",
                json!({"type": "object", "properties": {}}),
            ),
            ToolDescriptor::new(
                "fluid.set_config",
                "Update simulation configuration",
                json!({
                    "type": "object",
                    "properties": {
                        "viscosity": {"type": "number"},
                        "diffusion": {"type": "number"},
                        "dt": {"type": "number"},
                    },
                }),
            ),
            ToolDescriptor::new(
                "fluid.add_density",
                "Add density at a specific grid cell",
                json!({
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer"}, "y": {"type": "integer"},
                        "amount": {"type": "number"},
                    },
                    "required": ["x", "y", "amount"]
                }),
            ),
            ToolDescriptor::new(
                "fluid.sample",
                "Sample velocity and density at a point",
                json!({
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer"}, "y": {"type": "integer"},
                    },
                    "required": ["x", "y"]
                }),
            ),
            ToolDescriptor::new(
                "fluid.reset",
                "Reset the simulation to zero state",
                json!({"type": "object", "properties": {}}),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        match uri {
            "fluid://status" => {
                let sim = self.sim.read();
                ResourceContent::json(
                    uri,
                    &json!({
                        "width": sim.width(),
                        "height": sim.height(),
                        "config": {
                            "viscosity": sim.config().viscosity,
                            "diffusion": sim.config().diffusion,
                            "dt": sim.config().dt,
                        },
                        "stats": {
                            "density_sum": sim.density_grid().sum(),
                            "density_max": sim.density_grid().max(),
                        },
                    }),
                )
            }
            _ => Err(BridgeError::ResourceNotFound(uri.into())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        let mut sim = self.sim.write();
        match name {
            "fluid.step" => {
                sim.step();
                ToolResult::success_json(&json!({
                    "stepped": true,
                    "density_sum": sim.density_grid().sum(),
                }))
            }
            "fluid.set_config" => {
                let mut config = sim.config().clone();
                if let Some(v) = args.get("viscosity").and_then(|v| v.as_f64()) {
                    config.viscosity = v as f32;
                }
                if let Some(v) = args.get("diffusion").and_then(|v| v.as_f64()) {
                    config.diffusion = v as f32;
                }
                if let Some(v) = args.get("dt").and_then(|v| v.as_f64()) {
                    config.dt = v as f32;
                }
                sim.set_config(config.clone());
                ToolResult::success_json(&json!({
                    "config": {
                        "viscosity": config.viscosity,
                        "diffusion": config.diffusion,
                        "dt": config.dt,
                    }
                }))
            }
            "fluid.add_density" => {
                let x = args.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let y = args.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let amount = args
                    .get("amount")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;
                sim.add_density(x, y, amount);
                ToolResult::success_json(&json!({
                    "x": x, "y": y,
                    "density_added": amount,
                }))
            }
            "fluid.sample" => {
                let x = args.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let y = args.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let density = *sim.density_grid().get(x, y);
                let (vx, vy) = sim.velocity_at(x, y);
                ToolResult::success_json(&json!({
                    "x": x, "y": y,
                    "density": density,
                    "velocity_x": vx,
                    "velocity_y": vy,
                }))
            }
            "fluid.reset" => {
                sim.reset();
                ToolResult::success_json(&json!({"reset": true}))
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

/// AI bridge for hydraulic erosion simulation.
///
/// Exposes config, aggregate heightfield stats, and point sampling.
pub struct ErosionBridge {
    sim: Arc<RwLock<ErosionSim>>,
}

impl ErosionBridge {
    pub fn new(sim: Arc<RwLock<ErosionSim>>) -> Self {
        Self { sim }
    }
}

impl AiProvider for ErosionBridge {
    fn namespace(&self) -> &str {
        "erosion"
    }

    fn description(&self) -> &str {
        "Hydraulic erosion simulation control. Configure parameters, step, and \
         sample terrain height at specific points. Full heightmap not exposed."
    }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![ResourceDescriptor::json(
            "erosion://status",
            "Erosion Sim Status",
            "Grid dimensions, config, and terrain stats (min/max height, total water)",
        )]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor::new(
                "erosion.step",
                "Run one erosion step (or multiple with 'count')",
                json!({
                    "type": "object",
                    "properties": {
                        "count": {"type": "integer", "default": 1, "description": "Number of steps"},
                    },
                }),
            ),
            ToolDescriptor::new(
                "erosion.set_config",
                "Update erosion configuration",
                json!({
                    "type": "object",
                    "properties": {
                        "rain_rate": {"type": "number"},
                        "evaporation_rate": {"type": "number"},
                        "sediment_capacity": {"type": "number"},
                        "erosion_rate": {"type": "number"},
                        "deposition_rate": {"type": "number"},
                    },
                }),
            ),
            ToolDescriptor::new(
                "erosion.sample",
                "Sample terrain height, water, and sediment at a point",
                json!({
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer"}, "y": {"type": "integer"},
                    },
                    "required": ["x", "y"]
                }),
            ),
        ]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        match uri {
            "erosion://status" => {
                let sim = self.sim.read();
                ResourceContent::json(
                    uri,
                    &json!({
                        "width": sim.width(),
                        "height": sim.height(),
                        "config": {
                            "rain_rate": sim.config().rain_rate,
                            "evaporation_rate": sim.config().evaporation_rate,
                            "sediment_capacity": sim.config().sediment_capacity,
                            "erosion_rate": sim.config().erosion_rate,
                            "deposition_rate": sim.config().deposition_rate,
                        },
                        "stats": {
                            "height_min": sim.terrain().min(),
                            "height_max": sim.terrain().max(),
                            "water_sum": sim.water().sum(),
                        },
                    }),
                )
            }
            _ => Err(BridgeError::ResourceNotFound(uri.into())),
        }
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        let mut sim = self.sim.write();
        match name {
            "erosion.step" => {
                let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                for _ in 0..count {
                    sim.step();
                }
                ToolResult::success_json(&json!({
                    "steps": count,
                    "height_min": sim.terrain().min(),
                    "height_max": sim.terrain().max(),
                }))
            }
            "erosion.set_config" => {
                let mut config = sim.config().clone();
                if let Some(v) = args.get("rain_rate").and_then(|v| v.as_f64()) {
                    config.rain_rate = v as f32;
                }
                if let Some(v) = args.get("evaporation_rate").and_then(|v| v.as_f64()) {
                    config.evaporation_rate = v as f32;
                }
                if let Some(v) = args.get("sediment_capacity").and_then(|v| v.as_f64()) {
                    config.sediment_capacity = v as f32;
                }
                if let Some(v) = args.get("erosion_rate").and_then(|v| v.as_f64()) {
                    config.erosion_rate = v as f32;
                }
                if let Some(v) = args.get("deposition_rate").and_then(|v| v.as_f64()) {
                    config.deposition_rate = v as f32;
                }
                sim.set_config(config);
                ToolResult::success_json(&json!({"config_updated": true}))
            }
            "erosion.sample" => {
                let x = args.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let y = args.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                ToolResult::success_json(&json!({
                    "x": x, "y": y,
                    "terrain_height": sim.terrain().get(x, y),
                    "water_depth": sim.water().get(x, y),
                    "sediment": sim.sediment().get(x, y),
                }))
            }
            _ => Err(BridgeError::ToolNotFound(name.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_simulation::{FluidConfig, ErosionConfig, Grid2D};

    fn make_fluid() -> FluidBridge {
        let sim = FluidSim::new(32, 32, FluidConfig::default());
        FluidBridge::new(Arc::new(RwLock::new(sim)))
    }

    fn make_erosion() -> ErosionBridge {
        let heightmap = Grid2D::new(32, 32, 0.5f32);
        let sim = ErosionSim::new(heightmap, ErosionConfig::default());
        ErosionBridge::new(Arc::new(RwLock::new(sim)))
    }

    #[test]
    fn fluid_status() {
        let bridge = make_fluid();
        let content = bridge.read_resource("fluid://status").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["width"], 32);
    }

    #[test]
    fn fluid_step_and_sample() {
        let bridge = make_fluid();
        bridge
            .call_tool("fluid.add_density", json!({"x": 16, "y": 16, "amount": 100.0}))
            .unwrap();
        bridge.call_tool("fluid.step", json!({})).unwrap();
        let result = bridge
            .call_tool("fluid.sample", json!({"x": 16, "y": 16}))
            .unwrap();
        assert!(!result.is_error);
    }

    #[test]
    fn erosion_status() {
        let bridge = make_erosion();
        let content = bridge.read_resource("erosion://status").unwrap();
        let v: Value = serde_json::from_str(&content.text).unwrap();
        assert_eq!(v["width"], 32);
    }

    #[test]
    fn erosion_step() {
        let bridge = make_erosion();
        let result = bridge.call_tool("erosion.step", json!({"count": 5})).unwrap();
        assert!(!result.is_error);
    }
}
