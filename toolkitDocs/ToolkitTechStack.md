# Toolkit Tech Stack: Rust Creative Ecosystem

To build professional 2D/3D editing applications, we utilize a highly optimized, commercially viable, and memory-safe Rust library ecosystem. This document defines the curated dependencies packaged by the Velatura Toolkit.

To safeguard commercial use, the toolkit uses libraries with permissive licenses (MIT, Apache-2.0, or BSD) and explicitly avoids copyleft licenses (GPL/AGPL).

---

## 1. Core Library Selection

The following libraries form the backbone of the Velatura architecture.

### GPU Rendering & Computations

| Crate | License | Role |
|---|---|---|
| `wgpu` | MIT / Apache-2.0 | GPU abstraction layer (WebGPU standard) supporting Vulkan, Metal, DX12, and WebGPU. Handles both graphics rendering and compute shader pipelines. |
| `naga_oil` | MIT / Apache-2.0 | Dynamic shader composer. Allows importing shared WGSL files and performing conditional compilation (e.g., `#import`, `#if DEBUG`). |
| `encase` | MIT / Apache-2.0 | Enforces std140/std430 padding rules for CPU-GPU uniform/storage buffers at compile time, eliminating alignment crashes. |
| `wgpu-profiler` | MIT / Apache-2.0 | Profiling GPU time queries to isolate bottlenecks in compute passes and rendering steps. |
| `bevy_mikktspace` | MIT / Apache-2.0 | Tangent space calculation algorithm. Ensures tangent space calculations on meshes match industry-standard normal-map formats. |
| `block_compression` | MIT / Apache-2.0 | GPU-based texture compression (BC1-BC7), shifting encoding CPU workloads onto compute shaders to save PCIe bus transfer time. |

### User Interface & Viewports

| Crate | License | Role |
|---|---|---|
| `winit` | MIT / Apache-2.0 | Window management and hardware input polling loop. |
| `egui` | MIT / Apache-2.0 | Immediate-mode UI layout engine, optimized for low overhead. |
| `egui-wgpu` | MIT / Apache-2.0 | Integration layers connecting the egui UI pass with the main `wgpu` render loop. |
| `egui_dock` | MIT / Apache-2.0 | Dockable tab panel layouts (e.g., separating Viewports, Tool Panels, and Properties). |
| `egui_node_graph` | MIT / Apache-2.0 | Graph UI canvas for procedural and routing networks. |
| `egui-gizmo` | MIT / Apache-2.0 | 3D transform viewport widgets (Translate, Rotate, Scale). |

### Geometry & Spatial Ingestion

| Crate | License | Role |
|---|---|---|
| `gltf` | MIT / Apache-2.0 | Parser for importing and exporting glTF 2.0 models and binary buffers. |
| `parry3d` | Apache-2.0 | Bounding Volume Hierarchy (BVH) construction. Used on the CPU to accelerate raycasting and spatial queries. |
| `meshopt` | MIT | Geometry optimization (vertex cache optimization, vertex quantization, LOD simplification). |
| `guillotiere` | Apache-2.0 | 2D dynamic texture atlas packing. Organizes individual islands or bitmaps onto unified VRAM atlas sheets. |

### Color Science & Pipeline

| Crate | License | Role |
|---|---|---|
| `palette` | MIT / Apache-2.0 | Precise color-space conversions (sRGB, Linear RGB, OKLab, ACEScg). Replaces heavy C++ dependencies like OpenColorIO. |

### State & Communication

| Crate | License | Role |
|---|---|---|
| `rkyv` | MIT / Apache-2.0 | Zero-copy serialization. Memory-maps project files from disk straight into CPU memory, avoiding deserialization delays. |
| `undo` | MIT | Implements the Command Pattern for document undo/redo state stacks. |
| `crossbeam-channel` | MIT / Apache-2.0 | Multi-producer multi-consumer message passing to route state changes between UI and engine threads without blocking. |
| `petgraph` | MIT / Apache-2.0 | Directed Acyclic Graph (DAG) structures. Powers dependency tracking and invalidation passes in procedural node trees. |

---

## 2. Key GPU-Compute Optimization Strategies

Velatura shifts heavy operations to the GPU using compute shaders:

- **Tangent-Space Normal Generation:** Generating normals from painted height data dynamically. Compute shaders evaluate orthogonal partial derivatives over a height texture and normalize the output.
- **Jump Flooding Algorithm (JFA) for Texture Dilation:** When painting near UV seams or baking textures, edge bleeding is necessary to prevent black seams during mipmapping. Instead of expanding borders iteratively on the CPU, a compute shader implements the JFA to expand UV islands logarithmically on the GPU.
- **TDR Watchdog Mitigation:** Heavy compute shaders (e.g., raytraced ambient occlusion) can run for seconds, triggering the OS Timeout Detection and Recovery (TDR) which crashes the application. Velatura partitions compute dispatches into tiles (e.g., 512x512) and executes them across multiple frames, keeping the UI responsive.
