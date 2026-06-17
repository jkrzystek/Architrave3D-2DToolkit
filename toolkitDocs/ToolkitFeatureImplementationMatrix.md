# Toolkit Feature Implementation Matrix

This matrix defines the boundary between modules provided by the Velatura Toolkit (core features and integrated libraries) and the custom code a developer must write to build their specific editing application.

---

## 1. User Interface & Workspaces

| Feature Area | Provided by Toolkit & Libraries | Custom Code to Write (App Level) |
|---|---|---|
| **Docking Layouts** | Tab management, panel resizing, and layout saving (`egui_dock`). | Organizing specific tabs (e.g., placing the Layer Panel next to the Color Picker). |
| **Node Graph UI** | Node sockets, link dragging, and infinite canvas navigation (`egui_node_graph`). | Defining application-specific node types, custom parameters, and output shapes. |
| **3D Transform Gizmos** | Viewport Translate/Rotate/Scale handles (`egui-gizmo`). | Syncing camera matrices with the gizmo viewport and applying transforms to custom mesh nodes. |
| **Brush Cursor Previews** | Viewport decal overlays (3D) and basic cursor rings (2D) rendered on top of the viewport. | Animating cursor indicators or mapping custom stylus cursor shapes. |

---

## 2. Rendering & Viewports

| Feature Area | Provided by Toolkit & Libraries | Custom Code to Write (App Level) |
|---|---|---|
| **Graphics Context** | Multi-backend initialization and surface management (`wgpu`). | Handling custom fallback adapters or multi-GPU configurations. |
| **Viewport Offscreen Target** | Rendering viewport passes to textures and mapping them to `egui::TextureId`. | Resizing viewport targets dynamically when egui panels are resized. |
| **Forward PBR Viewer** | Base PBR shader code evaluating standard glTF GGX and Smith shading equations. | Writing custom viewport shaders or lighting profiles (e.g., stylized toon shading or matcaps). |
| **Environment IBL** | Compute shaders prefiltering equirectangular HDRIs into irradiance/specular cubemaps. | Designing HDR background skybox drawings or specialized sun/atmosphere systems. |

---

## 3. Painting & Brush Operations

| Feature Area | Provided by Toolkit & Libraries | Custom Code to Write (App Level) |
|---|---|---|
| **Decal Projection** | Screen-space decal projection shader mapping coordinates onto UV channels. | Managing application-specific channel combinations (e.g., painting only roughness). |
| **Normal Map Generation** | Sobel derivative compute passes converting height/depth maps to tangent-space normals. | Designing custom stamp height profiles or paint volume thresholds. |
| **Stroke Smoothing** | Spring-damper math tracking the cursor coordinate trajectory. | Tuning spring coefficients or adding custom stylus pressure response curves. |
| **Alpha Generation** | Procedural noise alphas generated directly in compute shaders. | Loading custom image-file alpha brushes from disk. |

---

## 4. Geometry & Baking

| Feature Area | Provided by Toolkit & Libraries | Custom Code to Write (App Level) |
|---|---|---|
| **Mesh Ingestion** | glTF 2.0 file parser and vertex array loading (`gltf`). | Loading legacy or proprietary formats (e.g., FBX, OBJ). |
| **Spatial Hierarchy** | Building BVH trees on the CPU and flattening them for GPU storage buffers (`parry3d`). | Handling dynamic deformation of the BVH if the mesh changes shape. |
| **Baking Passes** | Software raytraced AO, Thickness, and Bent Normal compute shaders in WGSL. | Setting up custom baking configuration panels and queueing bake steps. |
| **TDR Mitigation** | Slicing compute tasks into chunked tiles and managing execution over frames. | Balancing tile sizes to prioritize frame rates vs. baking speed. |

---

## 5. State, History & Export

| Feature Area | Provided by Toolkit & Libraries | Custom Code to Write (App Level) |
|---|---|---|
| **Undo / Redo** | Command pattern action execution and history stack bounds (`undo`). | Defining application-specific Command enums (e.g., `NurbsCommand` or `ErosionCommand`). |
| **Serialization** | Zero-copy memory-mapping serialization (`rkyv`). | Structuring project save/load layout directories. |
| **Channel Packing** | Compute shaders mapping multiple grayscale channels into single color channels (e.g., RGB). | Designing presets for external game engines (e.g., Unity HDRP or Unreal Engine). |
| **Texture Dilation** | Jump Flooding Algorithm compute shaders padding UV islands. | Applying dilation steps to custom export buffers. |
