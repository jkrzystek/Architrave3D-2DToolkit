# AI Bridge Guide

The `toolkit_ai_bridge` crate provides a protocol-agnostic layer for exposing toolkit functionality to AI/LLM systems. Currently implements MCP (Model Context Protocol) with a design that makes swapping to future protocols straightforward.

## Architecture

```
┌──────────────────────────────────────────────────┐
│                  LLM Client                       │
│         (Claude, GPT, or any MCP client)          │
└─────────────────────┬────────────────────────────┘
                      │ JSON-RPC over stdio
┌─────────────────────▼────────────────────────────┐
│               McpServer                           │
│         (MCP protocol handling)                   │
│         Translates MCP ↔ BridgeRegistry           │
└─────────────────────┬────────────────────────────┘
                      │
┌─────────────────────▼────────────────────────────┐
│            BridgeRegistry                         │
│     Collects all AiProvider implementations       │
│     Routes requests by namespace                  │
└──┬──────┬──────┬──────┬──────┬──────┬──────┬─────┘
   │      │      │      │      │      │      │
   ▼      ▼      ▼      ▼      ▼      ▼      ▼
Document Camera Geometry Graph  Fluid Erosion UI Scene
Bridge   Bridge  Bridge  Bridge Bridge Bridge   Bridge
   │      │      │      │      │      │      │
   ▼      ▼      ▼      ▼      ▼      ▼      ▼
Arc<RwLock<Document>>  etc. (shared app state)
```

## The AiProvider Trait

Every adapter implements this protocol-agnostic interface:

```rust
pub trait AiProvider: Send + Sync {
    fn namespace(&self) -> &str;
    fn description(&self) -> &str;
    fn list_resources(&self) -> Vec<ResourceDescriptor>;
    fn list_tools(&self) -> Vec<ToolDescriptor>;
    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent>;
    fn call_tool(&self, name: &str, arguments: Value) -> BridgeResult<ToolResult>;
}
```

- **Resources** are read-only data the LLM can inspect (document info, camera state, etc.)
- **Tools** are actions the LLM can perform (add layers, move camera, evaluate graph)

## Available Adapters

### DocumentBridge (`adapter-state` feature)

| Resources | Description |
|-----------|-------------|
| `document://info` | Name, dimensions, layer count, dirty state |
| `document://layers` | Full layer tree with properties |
| `document://active_layer` | Currently selected layer |

| Tools | Description |
|-------|-------------|
| `document.add_layer` | Create a new layer |
| `document.remove_layer` | Delete a layer by ID |
| `document.set_layer_opacity` | Set opacity 0.0-1.0 |
| `document.set_layer_visibility` | Show/hide layer |
| `document.rename_layer` | Change layer name |
| `document.set_active_layer` | Select a layer |

### CameraBridge (`adapter-render` feature)

| Resources | Description |
|-----------|-------------|
| `camera://state` | Position, target, projection settings |
| `camera://orbit` | Orbit controller yaw/pitch/distance |

| Tools | Description |
|-------|-------------|
| `camera.set_position` | Set camera world position (x,y,z) |
| `camera.set_target` | Set look-at point |
| `camera.orbit` | Rotate orbit by delta yaw/pitch |
| `camera.zoom` | Zoom in/out |

### GeometryBridge (`adapter-geometry` feature)

| Resources | Description |
|-----------|-------------|
| `geometry://meshes` | All meshes with vertex/triangle counts and bounding boxes |

| Tools | Description |
|-------|-------------|
| `geometry.create_primitive` | Create cube/plane/sphere |
| `geometry.raycast` | Cast ray against mesh, get hit info |
| `geometry.remove_mesh` | Remove a mesh by name |

### GraphBridge (`adapter-graph` feature)

| Resources | Description |
|-----------|-------------|
| `graph://overview` | Node count, dirty count |
| `graph://nodes` | All nodes with template, position, dirty |
| `graph://templates` | Available node templates with port definitions |

| Tools | Description |
|-------|-------------|
| `graph.add_node` | Add node by template name |
| `graph.remove_node` | Remove a node |
| `graph.connect` | Connect output→input ports |
| `graph.disconnect` | Remove a connection |
| `graph.set_input_value` | Set a float input value |
| `graph.evaluate` | Evaluate all dirty nodes |

### FluidBridge (`adapter-simulation` feature)

| Resources | Description |
|-----------|-------------|
| `fluid://status` | Grid size, config, aggregate stats |

| Tools | Description |
|-------|-------------|
| `fluid.step` | Advance simulation one step |
| `fluid.set_config` | Update viscosity/diffusion/dt |
| `fluid.add_density` | Add density at a cell |
| `fluid.sample` | Read density/velocity at a point |
| `fluid.reset` | Reset simulation to zero |

### ErosionBridge (`adapter-simulation` feature)

| Resources | Description |
|-----------|-------------|
| `erosion://status` | Grid size, config, terrain stats |

| Tools | Description |
|-------|-------------|
| `erosion.step` | Run erosion steps |
| `erosion.set_config` | Update erosion parameters |
| `erosion.sample` | Read height/water/sediment at a point |

### InputBridge (`adapter-input` feature)

| Resources | Description |
|-----------|-------------|
| `input://stabilizer` | Stabilizer config and active state |
| `input://stroke` | Current stroke metadata |

| Tools | Description |
|-------|-------------|
| `input.set_stabilizer` | Configure spring constant, damping, dead zone |

### UiBridge (`adapter-ui` feature)

| Resources | Description |
|-----------|-------------|
| `ui://theme` | Theme mode, colors, font sizes |
| `ui://panels` | All panels with visibility |

| Tools | Description |
|-------|-------------|
| `ui.set_theme_mode` | Switch dark/light |
| `ui.set_accent_color` | Set accent RGB |
| `ui.toggle_panel` | Show/hide a panel |

### SceneBridge (`adapter-scene` feature)

| Resources | Description |
|-----------|-------------|
| `scene://overview` | Node and root counts |
| `scene://nodes` | All nodes: id, name, kind, visibility, parent, translation |

| Tools | Description |
|-------|-------------|
| `scene.add_node` | Add a transform node (optionally under a parent) |
| `scene.remove_node` | Remove a node and its subtree |
| `scene.set_translation` | Set a node's local position |
| `scene.set_scale` | Set a node's local scale |
| `scene.set_visible` | Show/hide a node |
| `scene.reparent` | Move a node under a new parent (cycle-rejecting) |

Nodes are addressed by a stable `"index:generation"` id. Geometry is referenced by id only — never exposed as raw vertex data.

## Feature Flags

Control which adapters are compiled:

```toml
# Include everything (default)
toolkit_ai_bridge = { path = "..." }

# Only document and camera adapters
toolkit_ai_bridge = { path = "...", default-features = false, features = ["adapter-state", "adapter-render"] }
```

## Futureproofing: Replacing MCP

The `AiProvider` trait and `BridgeRegistry` are protocol-agnostic. If MCP is replaced by a new protocol:

1. Write a new server struct (like `McpServer`) that implements the new protocol
2. Have it call into the same `BridgeRegistry`
3. No changes needed to any adapters

The adapters only know about `AiProvider`; they never reference MCP types. This inversion means protocol changes are isolated to one file.

## Writing Custom Adapters

```rust
use toolkit_ai_bridge::*;

struct MyCustomBridge {
    state: Arc<RwLock<MyState>>,
}

impl AiProvider for MyCustomBridge {
    fn namespace(&self) -> &str { "custom" }
    fn description(&self) -> &str { "My custom module" }

    fn list_resources(&self) -> Vec<ResourceDescriptor> {
        vec![ResourceDescriptor::json(
            "custom://status",
            "Status",
            "Current state of my module",
        )]
    }

    fn list_tools(&self) -> Vec<ToolDescriptor> {
        vec![ToolDescriptor::new(
            "custom.do_thing",
            "Performs the thing",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "param": {"type": "string"}
                },
                "required": ["param"]
            }),
        )]
    }

    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        // Return semantic data, not raw buffers
        todo!()
    }

    fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
        // Validate args, perform action, return result
        todo!()
    }
}
```
