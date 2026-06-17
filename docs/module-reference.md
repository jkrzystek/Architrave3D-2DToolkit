# Module Reference

Detailed reference for each toolkit crate.

---

## toolkit_core

Foundation types shared by all other crates.

### Type-Safe IDs
`LayerId`, `TextureId`, `NodeId`, `MeshId`, `MaterialId`, `ViewportId` — generated via `define_id!` macro. Each uses a global `AtomicU64` counter. Methods: `new()`, `from_raw(u64)`, `raw() -> u64`. Implements `Debug`, `Display`, `Hash`, `Eq`, `Serialize`, `Deserialize`.

### Events
`ViewportInputEvent` — enum covering `PointerMoved`, `PointerPressed`, `PointerReleased`, `Scroll`, `PinchZoom`, `KeyPressed`, `KeyReleased`. Includes `StylusState` (pressure, tilt, rotation), `Modifiers`, `KeyCode`, `PointerButton`.

### Commands
- `DocumentCommand` — `AddLayer`, `RemoveLayer`, `SetLayerOpacity`, `SetLayerVisibility`, `SetLayerBlendMode`, `MoveLayer`, `RenameLayer`, `Undo`, `Redo`
- `RenderCommand` — `UploadTexture`, `RemoveTexture`, `InvalidateLayer`, `InvalidateViewport`, `ResizeViewport`, `SetClearColor`
- `BlendMode` — 15 modes: Normal, Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion, Hue, Saturation, Luminosity
- `LayerKind` — Paint, Fill, Folder, Mask, Adjustment
- `TextureFormat` — R8, Rg8, Rgba8, Rgba16Float, Rgba32Float, R32Float, Depth32Float

### CommandDispatcher
Trait-based dispatcher with `ChannelDispatcher` (crossbeam channels) and `MockDispatcher` for testing.

### TileMap\<T\>
Generic sparse tile container using `HashMap<IVec2, T>`. Methods: `pixel_to_tile()`, `tile_to_pixel_origin()`, `tiles_in_rect()`, `get_or_create()`.

### LinearRgba
Color type with sRGB ↔ linear conversion (standard transfer function), `lerp()`, premultiplied alpha, luminance calculation.

---

## toolkit_input

Input telemetry and stroke processing.

### InputBuffer
Ring buffer for high-frequency input samples. Configurable capacity (default 256). Methods: `push()`, `latest()`, `samples_since()`, `average_velocity()`, `average_pressure()`.

### StrokeStabilizer
Spring-damper "lazy mouse" algorithm. Config: `spring_constant`, `damping`, `dead_zone`, `enabled`. Call `reset(pos)` at stroke start, `update(cursor, dt)` each frame — returns `Some(smoothed_pos)` or `None` if within dead zone.

### Stroke
Recorded stroke of `StrokePoint`s (position, pressure, tilt, timestamp). Methods: `point_count()`, `duration_ms()`, `bounding_box()`, `resample(interval_px)` for even-spacing with linear interpolation.

---

## toolkit_state

Document model with layer tree and undo/redo.

### Document
Top-level struct: `root_layer` (always Folder), `active_layer_id`, `width`, `height`, `name`, `dirty`. Methods: `add_layer()`, `remove_layer()`, `find_layer()`, `find_layer_mut()`, `set_active_layer()`, `layer_count()`, `all_layers()`. JSON-serializable.

### Layer
Tree node: `id`, `name`, `kind`, `opacity`, `visible`, `blend_mode`, `locked`, `children`, `parent_id`. Methods: `is_folder()`, `add_child()`, `remove_child()`, `find()`, `find_mut()`, `depth_first_iter()`, `subtree_count()`.

### blend()
`blend(base: [f32;4], top: [f32;4], mode: BlendMode, opacity: f32) -> [f32;4]` — Porter-Duff alpha compositing with real blend mode math for all 15 modes.

### HistoryStack
`UndoAction` trait (apply, undo, description). Bounded stack with cursor navigation, redo-branch truncation. Methods: `push()`, `undo()`, `redo()`, `can_undo()`, `can_redo()`.

---

## toolkit_render

GPU abstraction via wgpu.

### GpuContext
Wraps wgpu `Device`, `Queue`, `AdapterInfo`. Async `new()` with `GpuContextDescriptor` (power preference, backends, limits). Methods: `create_shader_module()`, `create_buffer()`, `create_buffer_init()`, `backend_name()`.

### GpuTexture
Wraps wgpu `Texture` + `TextureView`. Created from `TextureDescriptor` with toolkit's `TextureFormat`. Method: `upload(context, data)`.

### TextureCache
LRU cache with frame-based eviction and byte budget (`max_bytes`). Call `advance_frame()` each frame; textures unused for N frames get evicted.

### Camera
`position`, `target`, `up`, `projection` (Perspective or Orthographic). Constructors: `perspective()`, `orthographic()`. Methods: `view_matrix()`, `projection_matrix()`, `view_projection()`.

### OrbitController
Fields: `distance`, `yaw`, `pitch`, `target`, min/max clamps. Methods: `rotate(dy, dp)`, `zoom(delta)`, `pan(dx, dy)`, `camera_position()`, `apply_to_camera()`.

### Uniforms
`ViewUniforms`, `ModelUniforms`, `LightUniforms` — `#[repr(C)]` `Pod`/`Zeroable`, 16-byte aligned. Ready for GPU upload via `bytemuck::bytes_of()`.

---

## toolkit_geometry

Mesh data structures and spatial queries.

### Vertex
`[f32; 3]` arrays for position, normal, tangent; `[f32; 2]` for uv. `Pod`/`Zeroable`. Stride: 48 bytes. Accessors: `position_vec3()`, `normal_vec3()`, `uv_vec2()`, `tangent_vec4()`.

### Mesh
`vertices: Vec<Vertex>`, `indices: Vec<u32>`, `name: String`. Methods: `triangle_count()`, `bounding_box()`. Generators: `Mesh::cube(size)`, `Mesh::plane(w, h, subdivs)`, `Mesh::uv_sphere(radius, segments, rings)`.

### Aabb
Axis-aligned bounding box: `min`, `max` (`Vec3`). Methods: `contains_point()`, `intersects()`, `surface_area()`, `extents()`, `from_points()`, `expand_to_include_aabb()`.

### Bvh
SAH-based BVH over triangle meshes. `Bvh::build(mesh)` constructs recursively. `bvh.intersect(ray, mesh)` returns closest `HitRecord`. `bvh.flatten()` produces `FlatBvh` for GPU traversal.

### Ray
Origin + normalized direction. `Ray::new(origin, dir)`, `ray.at(t)`.

### Intersection Functions
- `ray_triangle_intersection(ray, v0, v1, v2)` — Moller-Trumbore, returns `(t, u, v)`
- `ray_aabb_intersection(ray, aabb)` — slab method, returns `(t_near, t_far)`

---

## toolkit_graph

Procedural node graph with dirty propagation.

### NodeGraph
DAG backed by petgraph `StableDiGraph`. Methods: `add_node(template, pos)`, `remove_node(id)`, `connect(from, port, to, port)` (with cycle detection), `disconnect()`, `mark_dirty(id)` (BFS downstream), `topological_order()`, `dirty_nodes()`, `node()`, `node_mut()`, `all_nodes()`, `set_input()`, `connections_from()`, `connections_to()`.

### NodeTemplate
Trait: `name()`, `inputs()`, `outputs()`, `evaluate(inputs) -> outputs`. Built-in: `FloatConstant`, `AddFloat`, `MultiplyFloat`, `MixFloat`.

### NodeRegistry
`register(template)`, `get(name)`, `template_names()`.

### evaluate_graph()
Evaluates dirty nodes in topological order. For each: gathers inputs (from connections or defaults), calls template's `evaluate()`, stores in `cached_outputs`, clears dirty.

---

## toolkit_simulation

Physics simulations on 2D grids.

### Grid2D\<T\>
Row-major 2D array. Methods: `get(x,y)`, `set(x,y,v)`, `try_get()`, `get_mut()`, `fill()`, `swap()`, `data()`, `data_mut()`. For `f32`: `sample_bilinear()`, `min()`, `max()`, `sum()`, `add_scaled()`.

### FluidSim
Jos Stam "Stable Fluids" — semi-Lagrangian solver. Public fields: `velocity_x`, `velocity_y`, `density` (all `Grid2D<f32>`). Config: `viscosity`, `diffusion`, `dt`, `gauss_seidel_iterations`. Methods: `step()`, `add_density(x,y,amount)`, `add_velocity(x,y,vx,vy)`, `reset()`, `density_grid()`, `velocity_at(x,y)`, `width()`, `height()`, `config()`, `set_config()`.

### ErosionSim
Hydraulic erosion: rain → gradient flow → velocity → sediment transport → deposition → evaporation. Config: `rain_rate`, `evaporation_rate`, `sediment_capacity`, `deposition_rate`, `erosion_rate`, `min_slope`, `gravity`, `iterations`. Methods: `step()`, `run(iters)`, `reset(heightmap)`, `terrain()`, `water()`, `sediment()`, `heightmap()`, `water_map()`, `width()`, `height()`, `config()`, `set_config()`.

---

## toolkit_ui

egui-based interface components.

### ViewportPanel
Allocates an egui region for 3D/2D viewport rendering. Returns `ViewportResponse` with rect, size_changed, mouse position in local and normalized coordinates.

### WorkspaceLayout
Manages `PanelState`s (id, title, visible, closable). Default panels: 3D Viewport, 2D Canvas, Layers, Properties, Tools, Color Picker. Methods: `toggle_panel()`, `visible_panels()`.

### ToolkitTheme
Dark/light mode, accent color, viewport background, font sizes. Methods: `accent()`, `viewport_bg()`, `apply_to_egui()`.

### Widgets
`color_swatch()`, `slider_with_label()`, `percentage_slider()`, `toggle_button()`, `PropertyGrid` (row-based property editor with `float_row`, `bool_row`, `text_row`).

---

## toolkit_ai_bridge

AI/LLM integration layer. See [AI Bridge Guide](ai-bridge-guide.md) for full details.

### Core Types
- `AiProvider` trait — protocol-agnostic resource/tool interface
- `BridgeRegistry` — collects providers, routes requests
- `McpServer` — MCP JSON-RPC server over stdio

### Adapters
`DocumentBridge`, `CameraBridge`, `GeometryBridge`, `GraphBridge`, `FluidBridge`, `ErosionBridge`, `InputBridge`, `UiBridge` — each wraps module state in `Arc<RwLock<T>>` and exposes semantic operations.
