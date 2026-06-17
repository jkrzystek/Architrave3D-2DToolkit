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
`DocumentBridge`, `CameraBridge`, `GeometryBridge`, `GraphBridge`, `FluidBridge`, `ErosionBridge`, `InputBridge`, `UiBridge`, `SceneBridge` — each wraps module state in `Arc<RwLock<T>>` and exposes semantic operations.

---

## toolkit_scene

3D scene graph: a forest of transform nodes stored in a generational arena.

### Transform
TRS transform: `translation` (Vec3), `rotation` (Quat), `scale` (Vec3). Methods: `to_matrix()`, `from_matrix()`, `mul_transform()`, `transform_point()`, `right()`/`up()`/`forward()`. Constructors: `from_translation`, `from_rotation`, `from_scale`, `IDENTITY`.

### Scene
Arena of `SceneNode`s addressed by `NodeKey` (index + generation; stale keys are detected). Methods: `add_node()`, `add_child()`, `get()`/`get_mut()`, `set_parent()` (rejects cycles), `remove()` (subtree), `iter()`, `roots()`, `update_world_transforms()` (propagates parent→child), `world_transform()`. `NodeKey::from_raw_parts()` reconstructs handles from wire data.

### SceneNode / NodeData
Node has `name`, `transform`, `visible`, `data`, cached `world_matrix`. `NodeData` is `Empty`, `Mesh { mesh: MeshId, material: Option<MaterialId> }`, `Light(Light)`, or `Camera`.

### Light / Selection
`Light` with `LightKind` (Directional, Point{range}, Spot{range,inner,outer}), color, intensity. `Selection` is a node set with an active member (`select_only`, `add`, `toggle`, `remove`, `contains`).

---

## toolkit_topology

Half-edge mesh and topology editing — the adjacency layer indexed meshes lack.

### HalfEdgeMesh
Half-edge structure (`HalfEdge`, `HeVertex`, `HeFace`, `HeEdge`). Build: `from_mesh(&Mesh)`, `from_polygons(positions, faces)`. Query: `face_vertices()`, `vertex_neighbors()`, `vertex_valence()`, `is_boundary_edge()`, `is_boundary_vertex()`, `euler_characteristic()`. Convert: `to_mesh()` (fan-triangulates), `recompute_normals()`.

### Editing
`catmull_clark()` → all-quad subdivision (any polygon mesh). `loop_subdivide()` → triangle subdivision (returns `Result`, `TopologyError::NonTriangular` for quads). `flip_edge(edge)` → 2-2 triangle swap. `triangulated()` → fan-triangulate every face.

### MeshSelection
Independent vertex/edge/face `HashSet`s with a `SelectMode`. Marking UV seams = selecting edges. Methods: `toggle_edge()`, `has_face()`, `count()`, etc.

---

## toolkit_uv

UV unwrapping and atlas packing.

### LSCM
`unwrap_lscm(positions, triangles)` → `UnwrapResult` (per-vertex UVs in `[0,1]²`). Least Squares Conformal Maps: builds a sparse conformality system solved by `solver::solve_least_squares` (CGLS). `conformal_distortion()` validates the result (≈0 = angle-preserving).

### Projections
`project_planar(positions, Axis)`, `project_cylindrical`, `project_spherical`, `project_box` — fast, solver-free unwraps.

### Charts & Atlas
`segment_charts(positions, triangles, seams)` → `Vec<Chart>` (connected patches between seam edges; local vertex remap). `Chart::unwrap()` flattens with LSCM. `unwrap_charts()` does both. `pack_charts(&mut charts, margin)` arranges islands into one `[0,1]²` space (shelf packer); `pack_sizes()` for raw bounding-box sizes. `AtlasPlacement` carries the per-chart `offset`/`scale`.

### solver
`SparseMatrix` (triplet form, `mul`/`mul_transpose`) and `solve_least_squares` (conjugate-gradient least squares). Dependency-free.

---

## toolkit_gizmo

Renderer-agnostic transform gizmo.

### Gizmo
`Gizmo { origin, orientation, mode, config }`. `hit_test(ray, view_dir)` → `Option<GizmoHandle>`. Drag: `begin_drag(handle, ray, view_dir)`, `update_drag(ray, view_dir)` → `GizmoDelta`, `end_drag()`. `GizmoMode` (Translate/Rotate/Scale), `GizmoAxis` (X/Y/Z/XY/YZ/XZ/Screen). `GizmoDelta::{Translate(Vec3), Rotate(Quat), Scale(Vec3)}` is cumulative from drag start.

### math
`closest_param_on_line`, `closest_point_on_line`, `ray_line_distance`, `ray_plane_intersection` — the ray helpers the gizmo (and other pickers) need.

---

## toolkit_canvas

2D editor foundation (the "2D basics" used by the paint and UV editors).

### CanvasView
2D pan/zoom camera. `canvas_to_screen()`/`screen_to_canvas()`, `pan_pixels()`, `zoom_at(anchor, factor)` (cursor-anchored zoom), `fit_bounds(min, max, padding)`, `visible_bounds()`. Holds `center`, `zoom`, `viewport`, zoom clamps.

### grid
`adaptive_step(view, target_px)` → "nice" (1/2/5×10ⁿ) spacing that stays readable at any zoom. `grid_lines(view, step)` → visible line coordinates. `snap(point, step)`.

### selection
`Rect2` (AABB: `from_corners`, `contains`, `intersects`, `area`) and `SelectionDrag` (rubber-band: `begin`/`update`/`finish` → `Rect2`).

---

## toolkit_assets

Asset import/export with a format-neutral result.

### ImportedScene
`{ meshes: Vec<Mesh>, instances: Vec<MeshInstance> }`. `build_scene()` → `toolkit_scene::Scene` of nodes referencing the meshes by id. Every importer returns this, so downstream code is format-agnostic.

### OBJ
`import_obj_str()` / `import_obj_path()` (positions, UVs, normals, `o`/`g` groups, polygon fan triangulation, negative indices). `export_obj()` / `export_obj_path()`.

### glTF 2.0
`import_gltf_slice(bytes)` (`.glb` or self-contained `.gltf` with embedded base64 buffers) and `import_gltf_path()` (resolves external `.bin`). Flattens the node hierarchy to world-space instances. Backed by the `gltf` crate.

---

## toolkit_render (additions)

### PbrMaterial
Metallic-roughness PBR (glTF workflow): `base_color`, `metallic`, `roughness`, `emissive`+`emissive_strength`, `normal_scale`, `occlusion_strength`, alpha mask, double-sided, and optional texture-map `TextureId`s. Constructors: `dielectric()`, `metal()`. `uniforms()` → `MaterialUniforms` (16-byte-aligned `Pod` block with packed `MaterialFlags`). `PBR_SHADER_WGSL` is a reference Cook-Torrance (GGX) shader.

### Navigation
`FlyController` (first-person WASD + mouse-look: `look()`, `move_local()`, `apply_to()`, `from_camera()`). Framing: `frame_orbit()`, `frame_camera()`, `framing_distance()` focus the camera on a bounding sphere.
