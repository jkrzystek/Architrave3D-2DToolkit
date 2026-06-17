# Building Apps with the Toolkit

This guide shows how to use toolkit crates to build a complete application.

## Choosing Your Crates

Pick only the crates you need:

| Building... | You need |
|-------------|----------|
| 3D model viewer | `core`, `render`, `geometry`, `ui` |
| 2D paint app | `core`, `input`, `state`, `render`, `ui` |
| Node-based generator | `core`, `graph`, `geometry`, `render`, `ui` |
| Terrain editor | `core`, `state`, `render`, `geometry`, `simulation`, `ui` |
| Headless mesh processor | `core`, `geometry` |
| AI-accessible editor | Any of the above + `ai_bridge` |

## Minimal App Setup

### Step 1: Create Your App Crate

```bash
cargo init my-3d-app
```

```toml
# my-3d-app/Cargo.toml
[dependencies]
toolkit_core = { path = "../app-3d-toolkit/crates/toolkit_core" }
toolkit_state = { path = "../app-3d-toolkit/crates/toolkit_state" }
toolkit_render = { path = "../app-3d-toolkit/crates/toolkit_render" }
toolkit_ui = { path = "../app-3d-toolkit/crates/toolkit_ui" }
# ... add others as needed
```

### Step 2: Initialize Core Systems

```rust
use toolkit_core::*;
use toolkit_state::Document;
use toolkit_render::camera::{Camera, OrbitController};
use toolkit_ui::{ToolkitTheme, WorkspaceLayout};

fn main() {
    // Document model
    let mut document = Document::new("My Project", 1920, 1080);
    document.add_layer("Background", LayerKind::Paint, None);
    document.add_layer("Foreground", LayerKind::Paint, None);

    // Camera
    let camera = Camera::perspective(
        glam::Vec3::new(0.0, 5.0, 10.0),
        glam::Vec3::ZERO,
        45.0,
        16.0 / 9.0,
    );
    let orbit = OrbitController::default();

    // UI theme
    let theme = ToolkitTheme::default(); // dark mode
    let layout = WorkspaceLayout::default();

    // Start your render/event loop here...
}
```

### Step 3: Handle Input Events

```rust
use toolkit_core::{ViewportInputEvent, PointerButton};
use toolkit_input::{StrokeStabilizer, StabilizerConfig, Stroke, StrokePoint};

let mut stabilizer = StrokeStabilizer::new(StabilizerConfig::default());
let mut current_stroke = Stroke::new();

// In your event loop:
fn handle_event(event: ViewportInputEvent) {
    match event {
        ViewportInputEvent::PointerPressed { position, button, .. } => {
            if button == PointerButton::Primary {
                stabilizer.reset(position);
                current_stroke = Stroke::new();
            }
        }
        ViewportInputEvent::PointerMoved { position, .. } => {
            if let Some(smoothed) = stabilizer.update(position, dt) {
                current_stroke.push_point(StrokePoint {
                    position: smoothed,
                    pressure: 1.0,
                    tilt: glam::Vec2::ZERO,
                    timestamp_ms: now,
                });
            }
        }
        _ => {}
    }
}
```

### Step 4: Use the Command Dispatcher

```rust
use toolkit_core::{DocumentCommand, RenderCommand, BlendMode};

// Create dispatcher
let dispatcher = ChannelDispatcher::new(256);

// Send commands from input handling
dispatcher.send_document(DocumentCommand::AddLayer {
    name: "New Layer".into(),
    kind: LayerKind::Paint,
    parent: None,
});

// Process commands in your update loop
while let Some(cmd) = dispatcher.try_recv_document() {
    match cmd {
        DocumentCommand::AddLayer { name, kind, parent } => {
            document.add_layer(name, kind, parent);
        }
        DocumentCommand::Undo => { /* ... */ }
        _ => {}
    }
}
```

### Step 5: Work with Meshes and Geometry

```rust
use toolkit_geometry::{Mesh, Bvh, Ray};

// Create meshes
let terrain = Mesh::plane(100.0, 100.0, 64);
let cube = Mesh::cube(2.0);
let sphere = Mesh::uv_sphere(1.0, 32, 16);

// Build BVH for raycasting
let bvh = Bvh::build(&terrain);

// Raycast on click
let ray = Ray::new(camera.position, click_direction);
if let Some(hit) = bvh.intersect(&ray, &terrain) {
    println!("Hit at {:?}, normal {:?}", hit.position, hit.normal);
}
```

### Step 6: Node Graph (Procedural)

```rust
use toolkit_graph::*;

let mut graph = NodeGraph::new();
let mut registry = NodeRegistry::new();
registry.register(FloatConstant);
registry.register(AddFloat);
registry.register(MultiplyFloat);

// Build a graph: Constant(5) + Constant(3) = 8
let c1 = graph.add_node("FloatConstant".into(), (0.0, 0.0));
let c2 = graph.add_node("FloatConstant".into(), (0.0, 100.0));
let add = graph.add_node("AddFloat".into(), (200.0, 50.0));

graph.set_input(c1, 0, NodeValue::Float(5.0));
graph.set_input(c2, 0, NodeValue::Float(3.0));

graph.connect(c1, 0, add, 0).unwrap();
graph.connect(c2, 0, add, 1).unwrap();

evaluate_graph(&mut graph, &registry).unwrap();
let result = graph.node(add).unwrap().cached_outputs.as_ref().unwrap();
// result[0] == NodeValue::Float(8.0)
```

### Step 7: Simulation

```rust
use toolkit_simulation::*;

// Fluid simulation
let mut fluid = FluidSim::new(128, 128, FluidConfig::default());
fluid.add_density(64, 64, 100.0);
fluid.add_velocity(64, 64, 5.0, 0.0);
fluid.step();

// Erosion simulation
let heightmap = Grid2D::new(256, 256, 0.5);
let mut erosion = ErosionSim::new(heightmap, ErosionConfig::default());
erosion.run(100); // run 100 iterations
let terrain = erosion.terrain(); // read result
```

## Adding AI/LLM Access

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use toolkit_ai_bridge::*;
use toolkit_ai_bridge::adapters::*;

// Wrap state in Arc<RwLock<T>> for shared access
let document = Arc::new(RwLock::new(Document::new("My Doc", 1920, 1080)));
let camera = Arc::new(RwLock::new(camera));
let orbit = Arc::new(RwLock::new(orbit));

// Build the bridge registry
let mut registry = BridgeRegistry::new();
registry.register(DocumentBridge::new(document.clone()));
registry.register(CameraBridge::new(camera.clone(), orbit.clone()));
// ... add more adapters as needed

// Start MCP server (runs on stdio, blocks)
let mut server = McpServer::new(registry);
server.run_stdio().unwrap();
```

The MCP server runs on stdin/stdout, allowing any MCP-compatible LLM client to read and modify your app's state through semantic tools and resources.

## Architecture Tips

1. **Keep state centralized** in your `Document` — don't scatter layer data across multiple places
2. **Use the command dispatcher** for all state mutations — this makes undo/redo and MCP integration work automatically
3. **Wrap shared state in `Arc<RwLock<T>>`** if you need MCP access or multi-threaded rendering
4. **Don't depend on modules you don't use** — the toolkit is designed for a la carte usage
5. **Build the BVH once, query many times** — rebuild only when the mesh changes
