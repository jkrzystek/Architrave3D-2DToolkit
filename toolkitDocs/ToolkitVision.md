# Toolkit Vision: Velatura 2D/3D Creative App Toolkit

## 1. Core Vision & Philosophy

**Velatura** is a professional, high-performance, modular Rust toolkit designed for building applications that feature 2D and 3D graphics, viewports, and editing workflows. 

Rather than building a single monolithic editing application, the Velatura Toolkit provides a collection of decoupled, highly cohesive modules (crates) that developers can mix, match, and configure. By selecting only the required modules, developers can build a diverse array of graphics tools:
- **3D/2D Painting Applications** (incorporating PBR materials, fluid dynamics, and wet-on-wet paint simulation)
- **2D Drawing & Image Editors** (focusing on orthographic canvas painting, vector design, and photo filters)
- **NURBS-Based Editors** (providing vector-curve mathematical evaluation, surface lofting, and viewport manipulation)
- **SDF (Signed Distance Field) Modelers** (evaluating procedural shapes, CSG operations, and raymarching on the GPU)
- **Erosion & Terrain Simulators** (running physical simulation compute shaders over height fields and simulating soil advection)
- **Node-Based Procedural Generators** (composing procedural textures, materials, and geometry generators dynamically)

---

## 2. Structural Principles

To ensure commercial viability and architectural longevity, Velatura is built on three pillars:

1. **Pure Rust, Zero C/C++ Dependencies:** Avoiding the configuration overhead and compile-time fragility of C++ wrappers (e.g., Embree, OCIO, Qt). Everything from rendering (`wgpu`) to math (`glam`, `palette`), geometry (`parry3d`), and UI (`egui`) is implemented in safe, idiomatic, cross-platform Rust.
2. **Decoupled Engine Modules:** The core logic (state tracking, rendering, asset loading, etc.) is divided into clean, independent crates. A NURBS editor does not need to pay compilation or VRAM overhead for a fluid simulation compute shader, nor does an erosion simulator need 3D brush decals.
3. **Unidirectional Message-Passing Architecture:** Rather than sharing mutable state, applications built with the toolkit rely on a strict `Event -> Command -> Invalidation -> GPU Queue` pipeline. This keeps the UI responsive, renders viewport previews in a non-blocking loop, and makes undo/redo states mathematically deterministic.

---

## 3. The Reusable Module Stack

The toolkit is divided into modular building blocks that connect via standard interfaces:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        App Shell (Application)                         │
└────────┬──────────────────────────────────┬────────────────────┬───────┘
         │ (Events)                         │ (Commands)         │ (UI)
┌────────▼────────┐                 ┌───────▼────────┐   ┌───────▼───────┐
│  velatura_input │                 │ velatura_state │   │  velatura_ui  │
│  (Mouse/Stylus) │                 │ (Undo/Layers)  │   │  (egui/Dock)  │
└────────┬────────┘                 └───────┬────────┘   └───────────────┘
         │ (Viewport Coordinates)           │ (Render Commands)
         │                                  │
┌────────▼──────────────────────────────────▼────────────────────────────┐
│                             velatura_render                            │
│           (wgpu Backend, PBR Viewport, Compute Compositor)             │
└────────▲──────────────────▲──────────────────▲──────────────────▲───────┘
         │                  │                  │                  │
┌────────┴────────┐ ┌───────┴────────┐ ┌───────┴────────┐ ┌───────┴───────┐
│  velatura_brush │ │ velatura_graph │ │ velatura_geom  │ │ velatura_sim  │
│  (Decal Project)│ │ (naga_oil AST) │ │ (BVH/Raycast)  │ │ (Fluid/Erosion)
└─────────────────┘ └────────────────┘ └────────────────┘ └───────────────┘
```

- **`velatura_input`**: High-frequency telemetry capture (pressure, tilt, velocity) bypassing OS event queue bottlenecks.
- **`velatura_state`**: A unified document model, supporting multi-channel layers, project serialization via zero-copy memory mapping (`rkyv`), and a deterministic undo/redo stack.
- **`velatura_render`**: A raw `wgpu` wrapper providing standard PBR pipelines, viewport offscreen routing (rendering to textures used in immediate-mode UIs), and asynchronous resource queueing.
- **`velatura_brush`**: Screen-space decal projection pipelines that map mouse sweeps to GPU texture coordinates.
- **`velatura_graph`**: A node-graph execution engine that uses modular WGSL composition (`naga_oil`) to compile user-defined compute shaders on-the-fly.
- **`velatura_geometry`**: Spatial indexing (BVH construction on CPU via `parry3d`, flattened for GPU storage buffers) to handle raycasts and hit tests.
- **`velatura_simulation`**: Numerical solvers (Navier-Stokes, advection, Sobel height fields, terrain erosion) designed as modular compute passes.

---

## 4. Adaptability Scenarios (How to Build Different Apps)

### Scenario A: A 3D PBR Painter
- **Modules Used:** `velatura_input`, `velatura_state` (layers), `velatura_render` (PBR, offscreen), `velatura_brush` (decal projection), `velatura_geometry` (raycasting), `velatura_ui`, `velatura_simulation` (fluid paint mixing).
- **Wiring:** User draws on a 3D mesh. `velatura_input` feeds points to `velatura_geometry` to resolve coordinates. The resolved coordinates go to `velatura_brush` which runs decal compute shaders. `velatura_simulation` applies fluid advection on the texture, and `velatura_render` draws it using PBR.

### Scenario B: A 2D Painting App
- **Modules Used:** `velatura_input`, `velatura_state`, `velatura_render` (2D orthographic pass), `velatura_brush` (orthographic mapping), `velatura_simulation` (2D paint mixing), `velatura_ui`.
- **Wiring:** Bypasses `velatura_geometry` entirely. Input coordinates map directly to flat UV canvas space. Uses the same brush shaders and fluid simulation as the 3D app, demonstrating shared compute pipelines.

### Scenario C: A NURBS-Based Editor
- **Modules Used:** `velatura_input`, `velatura_state` (custom geometry nodes), `velatura_render` (line/wireframe pipeline), `velatura_geometry` (curve-point queries), `velatura_ui` (viewport gizmos).
- **Wiring:** Replaces paint layers with mathematical curve definitions. `velatura_render` draws curves procedurally on the GPU using custom line-rendering shaders, while `velatura_ui` allows editing anchor points with standard transform handles.

### Scenario D: An SDF Procedural Modeler
- **Modules Used:** `velatura_input`, `velatura_state` (SDF tree), `velatura_graph` (SDF operations), `velatura_render` (GPU Raymarching shader), `velatura_ui`.
- **Wiring:** User connects nodes in the UI. `velatura_graph` converts the tree of SDF functions into a combined WGSL shader. `velatura_render` compiles the shader and raymarches it onto a full-screen viewport quad.

### Scenario E: An Erosion Simulator
- **Modules Used:** `velatura_state` (height-field arrays), `velatura_render` (height map visualizer), `velatura_simulation` (thermal/hydraulic erosion compute shaders), `velatura_ui` (brush tools to add soil/water).
- **Wiring:** Runs continuous compute passes inside `velatura_simulation` to compute water velocity, sediment capacity, deposition, and evaporation, feeding the resulting height buffer to the viewport renderer.

---

## 5. Universal Connections

To connect these modules seamlessly, the toolkit defines standard interfaces:
- **`ViewportInputEvent`**: A unified input event schema for 2D/3D viewports.
- **`RenderCommand`**: A standard channel-based command schema to communicate between state managers and rendering queues.
- **`TileMap<T>`**: A tile-based spatial resource manager allowing scalable texture/matrix memory grids across 2D canvases, UDIM layouts, or terrain coordinates.
- **`CommandDispatcher`**: A trait allowing the UI or automation scripts to dispatch document updates without knowing the underlying execution backend.
