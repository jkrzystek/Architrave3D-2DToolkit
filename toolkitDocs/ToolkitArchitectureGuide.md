# Toolkit Architecture Guide: Velatura 2D/3D Framework

This document outlines the architecture for a modular, high-performance 2D/3D graphics application framework in Rust. It utilizes hardware-accelerated compute pipelines, non-destructive state management, and reusable UI modules to provide a foundation for creative software.

---

## 1. Domain Architecture (Modular Crates)

The framework is structured as a collection of decoupled libraries. Developers building applications select only the crates they need.

| Crate / Module | Role | Core Dependencies |
|---|---|---|
| `velatura_input` | Captures high-frequency input telemetry (stylus pressure, tilt, sub-pixel movements) and interfaces with windowing backends. | `winit` |
| `velatura_state` | Source of truth for document state. Manages layers, multi-tile maps (`TileMap`), history stacks, and project serialization. | `undo`, `rkyv` |
| `velatura_graph` | Non-destructive execution tree. Runs dirty-flag evaluation over dependency DAGs and compiles dynamically-generated shaders. | `petgraph`, `naga_oil` |
| `velatura_geometry` | CPU-side geometry processing. Imports 3D meshes, optimizes vertex layouts, and constructs spatial hierarchies for raycasting. | `parry3d`, `meshopt`, `gltf` |
| `velatura_render` | Low-level graphics backend. Evaluates PBR rendering, offscreen viewports, compute shader compositing, and GPU baking pipelines. | `wgpu`, `bevy_mikktspace`, `palette` |
| `velatura_ui` | Immediate-mode UI layout helpers. Provides docking systems, node graphs, viewport panels, and viewport transform gizmos. | `egui`, `egui_dock`, `egui_node_graph`, `egui-gizmo` |
| `velatura_simulation` | GPU-accelerated numerical simulation engine. Performs fluid advection, pigment mixing, and height-map/terrain erosion compute passes. | `wgpu` |

---

## 2. Unidirectional Data Flow

The toolkit enforces a strict unidirectional data flow. The user interface does not mutate GPU resources directly. Instead, UI events are turned into commands, processed by the state machine, which schedules render operations.

```
┌─────────────────┐      Event       ┌──────────────────┐
│   winit/egui    ├─────────────────►│  velatura_state  │
│  (Input/UI)     │                  │  (Document/Undo) │
└────────▲────────┘                  └────────┬─────────┘
         │                                    │
         │ Rendered Texture                   │ RenderCommand
         │                                    ▼
┌────────┴────────┐                  ┌──────────────────┐
│   egui-wgpu     │◄─────────────────┤  velatura_render │
│  (UI Render)    │   TextureId      │  (wgpu Queue)    │
└─────────────────┘                  └──────────────────┘
```

### High-Frequency Bypass (The Real-Time Track)
To prevent frame rate stuttering during high-frequency operations (such as brush strokes or camera manipulation), input coordinates are routed directly from `velatura_input` to the `velatura_render` queue:
1. `velatura_input` polls hardware telemetry at polling rate (e.g., 250Hz+ for styluses).
2. It sends `ViewportInputEvent`s directly to the rendering thread/pipeline.
3. The GPU runs coordinate projection and compute shader updates immediately.
4. Once the operation finishes, a low-frequency state command is sent to `velatura_state` to record the action for history and serialization.

---

## 3. Shared Projection and Viewport Pipelines

The rendering engine treats 2D drawing and 3D painting as different projections targeting a shared compute pipeline:

- **3D Viewport Decal Projection:** Reconstructs the world-space position under the cursor by sampling the depth buffer using the inverse view-projection matrix. An Oriented Bounding Box (OBB) is projected onto the mesh UVs to locate affected texels.
- **2D Standalone Canvas:** Orthographically maps screen coordinates directly to pixel positions on a flat texture.
- **2D UV Space Viewport:** Renders the 3D mesh wireframe flat in UV coordinates. Drawing maps the screen mouse coordinates directly to the underlying texture UV coordinate.

By sharing the underlying compute pipelines, features like procedural brushes, pigment mixing, and fluid advection behave identically in both 2D and 3D views.

---

## 4. State Management and Zero-Copy Serialization

Creative applications handle massive datasets (e.g., layers of 4K textures, high-density vertex buffers). To keep memory overhead and save/load times low:
- **`rkyv` Zero-Copy Serialization:** The document data model is saved using `rkyv`. When loading, the file is memory-mapped (`mmap`) into system RAM. The OS pages texture and vertex buffers on-demand, bypassing serialization/deserialization CPU bottlenecks.
- **VRAM LRU Cache:** The GPU VRAM should not hold all texture layers. `velatura_render` maintains an LRU VRAM cache. If VRAM is full, unused texture tiles are evicted. If they are needed again for rendering or compositing, they are paged back from the memory-mapped CPU buffers and re-uploaded.

---

## 5. Architectural Rules for Framework Consumers

When building applications with Velatura, developers must follow these strict rules:

1. **No GPU Handles in History:** The undo stack must never hold GPU resource handles (`wgpu::Texture`, `wgpu::Buffer`). Instead, store lightweight stroke paths or CPU-side pixel diff buffers (`Vec<u8>`). Recreate or upload to GPU resources on demand.
2. **Decoupled Viewports:** UI elements (`velatura_ui`) must never invoke GPU commands directly. They must issue `RenderCommand`s or `DocumentCommand`s which are queued and processed sequentially.
3. **TDR Crash Mitigation:** When running heavy GPU compute shaders (such as baking ambient occlusion or processing complex physical simulations), subdivide the work into tiles (e.g., 512x512) and queue them over several frames. This yields time back to the windowing event loop and prevents the OS from killing the graphics driver due to a Timeout Detection and Recovery (TDR) trigger.
