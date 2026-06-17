# 3dRustToolkit Architecture

## Overview

The toolkit is a Cargo workspace of independent, composable crates. Each crate handles one concern and can be used standalone or combined with others to build 2D/3D editor applications.

```
app-3d-toolkit/
├── Cargo.toml              (workspace root)
├── crates/
│   ├── toolkit_core        Foundation types (IDs, events, commands, color, tile map)
│   ├── toolkit_input       Input telemetry, stroke stabilizer, stroke recording
│   ├── toolkit_state       Document model, layer tree, blend modes, undo/redo
│   ├── toolkit_render      GPU context, textures, camera, pipelines, uniforms
│   ├── toolkit_geometry    Vertex, mesh, AABB, BVH, ray intersection
│   ├── toolkit_graph       Procedural node graph (DAG), evaluation engine
│   ├── toolkit_simulation  Fluid simulation, hydraulic erosion
│   ├── toolkit_ui          egui viewport, panels, theme, widgets
│   └── toolkit_ai_bridge   AI/LLM integration (MCP server, module adapters)
├── docs/                   This documentation
└── toolkitDocs/            Original design documents
```

## Dependency Graph

```
toolkit_core  (foundation, no internal deps)
     │
     ├── toolkit_input
     ├── toolkit_state
     ├── toolkit_render
     ├── toolkit_geometry
     ├── toolkit_graph
     ├── toolkit_simulation
     ├── toolkit_ui
     └── toolkit_ai_bridge  (depends on all above, with feature flags)
```

All crates depend on `toolkit_core` for shared types (IDs, events, commands). No other inter-crate dependencies exist — this is intentional to keep modules decoupled.

`toolkit_ai_bridge` is the exception: it depends on all modules (via optional feature flags) to provide AI/LLM access adapters.

## Key Design Principles

### 1. Unidirectional Data Flow
```
User Input → Event → Command → State Change → Invalidation → GPU Queue → Render
```
Input events flow through the command dispatcher, modify state, mark dirty regions, and trigger re-rendering. State is never modified directly by the render pipeline.

### 2. Message-Passing Architecture
Three communication tracks (from `toolkit_core::dispatcher`):
- **Input track** (bounded SPSC) — high-frequency pointer/stylus events
- **Document command track** (MPSC) — layer operations, undo/redo
- **Render command track** (MPSC) — texture uploads, viewport invalidation

### 3. Type-Safe IDs
All entity references use distinct newtype IDs (`LayerId`, `TextureId`, `NodeId`, `MeshId`, `MaterialId`, `ViewportId`). These are generated from a global atomic counter and cannot be mixed across types.

### 4. GPU-Friendly Data
Vertex layout uses `[f32; 3]` arrays (not `Vec3`) for `Pod`/`Zeroable` compatibility with `bytemuck`. Accessor methods (`position_vec3()`, `normal_vec3()`) convert to `glam` types for math.

### 5. Protocol-Agnostic AI Bridge
The AI integration uses a trait-based abstraction (`AiProvider`) that is independent of any specific protocol. MCP is one implementation; future protocols only require a new server implementation, not changes to adapters.

## Module Summary

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `toolkit_core` | Shared foundation | `LayerId`, `ViewportInputEvent`, `DocumentCommand`, `BlendMode`, `TileMap<T>`, `LinearRgba` |
| `toolkit_input` | Input processing | `InputBuffer`, `StrokeStabilizer`, `Stroke`, `StrokePoint` |
| `toolkit_state` | Document model | `Document`, `Layer`, `HistoryStack`, `UndoAction`, `blend()` |
| `toolkit_render` | GPU abstraction | `GpuContext`, `GpuTexture`, `TextureCache`, `Camera`, `OrbitController`, `ViewUniforms` |
| `toolkit_geometry` | Mesh & spatial | `Vertex`, `Mesh`, `Aabb`, `Bvh`, `Ray`, `HitRecord` |
| `toolkit_graph` | Procedural nodes | `NodeGraph`, `NodeTemplate`, `NodeRegistry`, `evaluate_graph()` |
| `toolkit_simulation` | Physics | `FluidSim`, `ErosionSim`, `Grid2D<T>` |
| `toolkit_ui` | Interface | `ViewportPanel`, `WorkspaceLayout`, `ToolkitTheme`, `PropertyGrid` |
| `toolkit_ai_bridge` | AI integration | `AiProvider`, `BridgeRegistry`, `McpServer`, per-module adapters |
