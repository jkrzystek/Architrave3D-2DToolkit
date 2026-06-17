# Toolkit Core Modules: API & Feature Inventory

The Velatura Toolkit organizes its functionalities into independent crates and modules. This document catalogs the APIs and features provided by each module, which developers can combine to create customized graphic editors.

---

## 1. `velatura_render` (Viewport & Rendering Module)

Responsible for initializing GPU pipelines and rendering graphic scenes.

- **PBR Render Pass:** Evaluates standard glTF 2.0 PBR materials: Base Color, Metallic, Roughness, Normal, Height (Displacement), and Ambient Occlusion.
- **Image-Based Lighting (IBL):** Compute shaders to prefilter HDRI environments into specular and diffuse map layers.
- **Dual-Viewport Synchronization:** Routines to link a 3D perspective camera viewport with a flat 2D UV canvas viewport, synchronizing coordinate transformations.
- **Offscreen Viewport Target:** Renders the GPU viewport to a texture buffer, which is exposed to immediate-mode UIs (`egui::TextureId`).
- **Optimization Views:** Fast viewport shaders (Unlit, Matcap, or Base Color only) for low-spec hardware profiles.
- **Material Preview Sphere:** Offscreen pass rendering material states onto a sphere/cube target to update UI browser thumbnails.

---

## 2. `velatura_brush` (Brush & Decal Module)

Calculates mouse/stylus inputs and updates raster resources.

- **Decal Projection Compute Pass:** WGSL compute shader projecting a 2D brush stamp/texture onto mesh UV channels using depth-buffer values.
- **Multi-Channel Painting:** Paints Color, Roughness, Metallic, and Height simultaneously in a single compute shader invocation.
- **Height-to-Normal Accumulator:** Real-time Sobel filtering computing tangent-space normal updates from newly painted height data.
- **Procedural Brush Generator:** A library of noise shaders allowing real-time parametric synthesis of brush stamps.
- **UDIM/Multi-Tile Routing:** Coordinates brush stroke stamps across texture tile borders, splitting a single brush stamp into multiple target tiles.

---

## 3. `velatura_state` (Document & Layer Stack Module)

Maintains application document data structure and editing history.

- **Layer Tree Structure:** Struct definitions for Layer Folders, Paint (raster) Layers, Fill (procedural) Layers, and Layer Masks.
- **Blending Compositor:** Multi-channel blend modes (Multiply, Screen, Overlay, Pass-through) evaluated bottom-to-top.
- **Height-Lerp Blending:** Advanced blend mode using height/displacement maps to deposit pigments or textures inside the crevices of lower layers.
- **Undo / Redo Stack:** Implements command history tracking using CPU state snapshots or stroke polylines.
- **Data Relays:** Mechanism to tag a layer channel's output as an input parameter for layer modifiers higher in the stack.

---

## 4. `velatura_graph` (Procedural Engine Module)

Manages node-based editing and custom shader compilation.

- **Shader Composition (`naga_oil`):** Compiles dynamically connected node-graph logic into a single, clean WGSL compute shader at runtime.
- **DAG Traversal (`petgraph`):** Invalidation system using dirty flags. Only recalculates graph nodes downstream of a user modification.
- **Procedural Noise Library:** Pre-packaged noise generators (Perlin, Worley, Simplex, Voronoi) optimized for GPU compute passes.

---

## 5. `velatura_geometry` (Geometry & Baking Module)

Ingests 3D assets and extracts ambient/geometric properties.

- **Mesh Loader:** Reads glTF 2.0 binary and text models.
- **BVH Construction:** Builds spatial indexing trees using `parry3d` on the CPU, flattening the tree into a structure format suitable for GPU compute shaders.
- **Raytraced Baking Passes:** Software raytracer executing in WGSL computing Ambient Occlusion, Thickness, and Bent Normals.
- **Analytical Baking Passes:** Shaders computing Curvature, World-Space Normals, Position, and Material ID maps.
- **TDR Dispatch Manager:** Automates chunked dispatches (e.g., in 512x512 blocks) to prevent OS watchdog drivers from timing out.

---

## 6. `velatura_simulation` (Numerical Simulation Module)

Provides physics solvers to simulate dynamic changes.

- **Fluid Solver:** Grid-based semi-Lagrangian advection computing fluid flow velocities.
- **Pigment Mixing (Kubelka-Munk):** Spectral upsampling and absorption/scattering evaluations to simulate wet-on-wet paint blending.
- **Terrain Erosion:** Hydraulic and thermal erosion solvers simulating sediment transport, deposition, and evaporation over heightfields.

---

## 7. `velatura_input` & `velatura_ui` (Input & Interface Modules)

Manages user interaction and UI layouts.

- **Input Telemetry Capture:** High-frequency event listener polling pen pressure, tilt, and sub-pixel coordinates.
- **Stroke Stabilizer:** Lazy-mouse spring-damper algorithm to smooth out coordinate jitter.
- **Dockable Docking Manager:** Standard interface layouts for docking viewports, layers, color pickers, and timeline history panels.
- **3D Transform Gizmos:** Interface hooks to position, scale, and rotate objects in the 3D viewport.
