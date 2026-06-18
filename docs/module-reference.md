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

## toolkit_attributes

Named, typed attribute channels bound to a geometry domain (point, vertex, edge, face, primitive, or global detail). This is the procedural-attribute backbone: instead of hard-coding vertex data layout, tools attach arbitrary named channels. The same machinery stores sculpt masks, paint weights, soft-selection falloff, per-element sim state, and extra UV sets.

### Attribute
One columnar channel of a single `AttributeType` (Float, Int32, Vec2, Vec3, Vec4, Color). Stores data as a typed `Vec`. Methods: `len()`, `get(n)`, `set(n, val)`, `push(val)`, `data_slice()`.

### AttributeSet
All channels for one domain, kept at the same length. `create(name, type)`, `get(name)`, `remove(name)`, `len()`, `resize(n)`.

### AttributeStore
One `AttributeSet` per domain (Point, Vertex, Edge, Face, Primitive, Detail). This is the bundle geometry carries. Methods: `create(domain, count, name, type)`, `get(domain, name)`, `resize_domain(domain, n)` — can infer count from other domains.

---

## toolkit_volume

Dense 3D grids of scalars or vectors (`Volume<T>`), placed and sampled in world space. Each lattice point stores a `T` (any `VolumeSample`: f32, Vec2, Vec3, Vec4). This is the 3D counterpart to `toolkit_simulation::Grid2D`, used by voxel sculpting, 3D fluids/erosion, SDF/density fields, and volume baking.

### Volume\<T\>
`new(size: [usize; 3], origin: Vec3, cell_size: Vec3)` or `from_fn(size, origin, cell_size, f)`. Methods: `sample(point)`, `gradient(point)` (scalar only), `resample(new_size)`, `lerp(&other, t)`, `size()`, `origin()`, `cell_size()`, `world_to_grid()`, `grid_to_world()`, `data()`.

### VolumeSample trait
Implemented by `f32`, `Vec2`, `Vec3`, `Vec4`. Controls how interpolation and gradient operate on the cell type.

---

## toolkit_field

A dependency-light `Field` trait mapping a point to a scalar and a `VectorField` mapping a point to `Vec3`, plus combinators that chain them without allocation. Closures implement `Field` for free, so any sampler (noise, SDF, volume, image) becomes a field with no wrapper.

### Field trait (and VectorField)
`fn sample(&self, p: Vec3) -> f32` / `fn sample(&self, p: Vec3) -> Vec3`. Implemented for closures and the built-in types.

### FieldExt combinators
Methods on any `Field`: `.add(other)`, `.mul(other)`, `.min(other)` / `.max(other)` (SDF union/intersection), `.clamp(lo, hi)`, `.remap(in_lo, in_hi, out_lo, out_hi)`, `.warp(field)`, `.translate(offset)`, `.scale(s)`. Chains without allocation.

### Built-in fields
`Constant(f32)`, `Sphere { center, radius }` (SDF sphere), `Plane { normal, d }` (SDF half-space). `gradient(field, p, eps)` numerically differentiates any field.

---

## toolkit_solver

Sparse matrices and iterative linear solvers, dependency-free. Assemble a system as a `SparseMatrix` in triplet form, then solve it. Generalises the solver that previously lived inside `toolkit_uv` so fluids, deformers, and unwrapping all share it.

### SparseMatrix
Triplet (COO) format: `new(rows, cols)`, `push(row, col, value)`, `mul(vector)`, `mul_transpose(vector)`. Convertible to CSR for efficient iteration.

### Solvers
- `solve_cg(&matrix, &rhs, max_iters, tol)` — conjugate gradient for SPD systems (Laplacian smoothing, pressure projection)
- `solve_least_squares(&matrix, &rhs, max_iters, tol)` — CGLS for any least-squares problem (UV unwrapping, gradient-domain)
- `solve_gauss_seidel(&matrix, &rhs, max_iters, tol)` — relaxation
- `solve_jacobi(&matrix, &rhs, max_iters, tol)` — relaxation

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
`SparseMatrix` (triplet form, `mul`/`mul_transpose`) and `solve_least_squares` (conjugate-gradient least squares). Dependency-free. *(Note: new projects should prefer `toolkit_solver` for solver needs — `toolkit_uv` keeps its embedded copy for backwards compatibility.)*

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

---

## toolkit_brush

A geometry-agnostic brush engine: falloff profiles and stroke stamping. A `Brush` (radius, strength, `Falloff`, dab spacing) turns a dragged path into evenly spaced dabs and reports a stroke weight at any point. It never touches geometry itself — callers multiply the returned weights into whatever they edit: sculpting, mesh painting, terrain editing, and weight painting all share one engine.

### Brush
`new(radius, strength)` → `Brush`. Fields: `radius`, `strength`, `falloff` (Falloff enum), `spacing`. Methods: `dab_centers(&[path_points])` → dab positions along the path, `stroke_weight(&dabs, query_point)` → f32 weight for a point under the stroke.

### Falloff
Enum: `Constant`, `Linear`, `Smooth` (3x²−2x³, the default), `Smoother` (6x⁵−15x⁴+10x³), `Sphere` (√(1−x²)). Each maps a normalised distance-from-dab-center [0,1] to a weight.

---

## toolkit_select

Weighted (soft) selection sets, element-kind agnostic. A `Selection` maps element indices to weights in [0,1]. Hard selections (all weights=1) and soft selections (falloff weights for proportional editing) use the same type. Complements `toolkit_topology::MeshSelection` (hard, mode-bound) by adding weights, boolean ops, grow/shrink, attribute thresholding, and distance falloff.

### Selection
`from_indices(indices)` / `from_weights(pairs)`. Methods: `contains(idx)`, `weight(idx)`, `count()`, `add(idx, weight)`, `remove(idx)`, `clear()`, `grow(&adjacency)` / `shrink(&adjacency)`, `union(&other)`, `intersect(&other)`, `threshold(weight)`, `select_by_attribute(&attrs, op, value)`.

### Adjacency
Build from edge pairs: `from_pairs(n_vertices, &[(u32, u32)])`. Query: `neighbors(vertex)` → slice of neighbor indices. Used by `Selection::grow`/`shrink`.

---

## toolkit_polyline

Polyline operations shared by pen tools, paint strokes, contours, and curve editing — written once over a `Point` trait so they serve 2D and 3D alike.

### Operations
- `length(points)` → total arc length
- `resample(points, n)` / `resample_by_spacing(points, spacing)` → even spacing along the curve
- `smooth_chaikin(points, iterations)` → corner-cutting subdivision
- `smooth_laplacian(points, iterations, lambda)` → relaxation smooth
- `simplify(points, epsilon)` → Douglas-Peucker point reduction
- `offset_2d(points, distance)` → sideways offset / outline of a 2D polyline

### Point trait
Implemented for `Vec2` and `Vec3`. Only `offset_2d` requires 2D; all other ops work on 3D polylines too.

---

## toolkit_meshedit

Poly-modeling operators on polygon meshes — the bread and butter of "normal" (manual) modeling. Operations work on an `EditMesh` (positions + face loops) that round-trips through `toolkit_topology::HalfEdgeMesh`, so each operator is a small, robust face-list edit rather than fragile half-edge surgery.

### EditMesh
`from_halfedge(&HalfEdgeMesh)` / `to_mesh(name)` → round-trip. Methods:
- `extrude_face(face_idx, distance)` / `extrude_faces(indices, distance)`
- `inset_face(face_idx, amount)`
- `bevel_face(face_idx, amount, segments)` — inset + extrude with segments
- `bridge_faces(face_a, face_b)` — connect two open boundary caps
- `dissolve_edge(edge_idx)` — merge two adjacent faces
- `fill_hole(boundary_halfedge)` / `fill_all_holes()` — cap open boundaries
- `loop_cut(edge_idx, param)` — insert an edge loop across a quad strip
- `face_count()`, `vertex_count()`

---

## toolkit_triangulate

2D polygon triangulation for procedural shapes, CAD profile faces, and filled text/vector outlines.

### Functions
- `triangulate(&polygon)` — ear-clips a simple polygon (handles concave and either winding)
- `triangulate_with_holes(&outer, &holes)` — bridges holes into the outer loop, then clips; returns `Triangulation { vertices, triangles }`
- `triangulate_delaunay(&polygon)` — ear-clip + constrained Lawson flip pass for Delaunay quality (fewer slivers)
- `signed_area(&polygon)` — positive = CCW winding
- `point_in_triangle(p, a, b, c)` — barycentric test

---

## toolkit_surfacing

Generate meshes from 2D profiles and 3D paths — the CAD/procedural surfacing kit. All functions return a `toolkit_geometry::Mesh` with smooth normals.

### Functions
- `extrude(profile, depth, caps)` — sweep a closed 2D profile along +Z
- `revolve(profile_points, segments)` — rotate a (radius, height) profile around Y
- `loft(cross_sections)` — surface through a sequence of 3D cross-section loops
- `sweep(profile, path, segments)` — extrude a 2D profile along a 3D path using rotation-minimizing frames

### Helpers
- `surface_from_grid(rows, cols, positions)` — build a mesh from a rectangular grid of Vec3s
- `finish_mesh(positions, indices)` — compute normals and wrap in Mesh

---

## toolkit_voxelize

Turn triangle meshes into volumes — the bridge from surfaces into `toolkit_volume` for remeshing, volume booleans, and simulations on arbitrary shapes.

### Functions
- `signed_distance_field(mesh, config)` → `Volume<f32>` — per-lattice unsigned distance to the nearest triangle, signed by ray-crossing parity inside test
- `solid(mesh, config)` → `Volume<f32>` — threshold the SDF into binary occupancy (0=outside, 1=inside)
- `surface_shell(mesh, config, shell_width)` → `Volume<f32>` — the thin band around the surface

### VoxelizeConfig
`resolution` (grid cells along the longest axis), `padding` (extra space around the mesh).

---

## toolkit_remesh

Mesh simplification and remeshing.

### Functions
- `decimate_to(mesh, target_tris)` / `decimate_ratio(mesh, ratio)` — QEM (quadric error metric) edge collapse. Removes triangles while preserving shape. The standard LOD / sculpt-cleanup simplifier.
- `cluster_remesh(mesh, cell_size)` — fast vertex clustering onto a uniform grid for unifying resolution and welding duplicate geometry.

### Quadric
The error metric type used by decimation. `Quadric::from_plane(normal, d)`, `Quadric::from_triangle(v0, v1, v2)`, `add`, `mul`, `evaluate(point)`.

---

## toolkit_meshops

Mesh utility operations that don't need a half-edge structure — the cleanup/processing steps you reach for constantly after generating or importing geometry.

### Functions
- `weld_vertices(mesh, epsilon)` — merge coincident vertices (de-duplicate soup)
- `recompute_normals(mesh)` / `recompute_tangents(mesh)` — regenerate shading attributes
- `flip_winding(mesh)` — reverse triangle orientation
- `merge(meshes)` — combine multiple meshes into one
- `stats(mesh)` → `MeshStats` — vertex/triangle count, bounding box, surface area, genus
- `laplacian_smooth(mesh, iterations, lambda)` — relax vertex positions
- `decimate_grid(mesh, cell_size)` — simplify by vertex clustering *(note: prefers `toolkit_remesh::decimate_*` for quality)*

---

## toolkit_anim

Keyframe animation. Build `Track`s of timed values (any `Animatable`: f32, Vec3, Quat), group them into a `TransformAnimation`, and advance an `AnimationPlayer` to drive `toolkit_scene` node transforms over time. Per-keyframe easing comes from `toolkit_easing`.

### Track\<T\>
`new()`, `add(time, value)`, `evaluate(time)` → interpolated value. Clamped to the keyframe range.

### TransformAnimation
Holds `translation: Track<Vec3>`, `rotation: Track<Quat>`, `scale: Track<Vec3>`. `sample(time)` → `Transform`. `apply_to_node(scene, node, time)` convenience method.

### AnimationPlayer
`new(duration)`, `update(dt)`, `play()`, `pause()`, `stop()`, `seek(time)`. `time` field for sampling. `finished()` when the end is reached.

### Re-exports
`toolkit_easing::Easing` — each keyframe can use a different easing curve.

---

## toolkit_curves

Parametric curves and surfaces — the foundation for CAD-style modeling. Curves tessellate to polylines; surfaces tessellate to a `toolkit_geometry::Mesh`.

### Curves
- `Bezier` — arbitrary-degree Bézier (de Casteljau). `new(control_points)`, `evaluate(t)`, `tessellate(segments)`, `split(t)`, `derivative(t)`
- `BSplineCurve` — B-spline with clamped/uniform or custom knots (De Boor). `new(points, degree)`, `evaluate(t)`, `tessellate(segments)`, `derivative(t)`
- `NurbsCurve` — rational B-spline (weights for circles/conics). Same API + `weights`
- `CatmullRom` — interpolating spline through waypoints. `new(points, alpha)`, `evaluate(t)`, `tessellate(segments)`

### Surfaces
- `NurbsSurface` — rational B-spline surface. `evaluate(u, v)`, `tessellate(u_segments, v_segments)` → Mesh

### Utilities
`clamped_uniform_knots(n, degree)`, `find_span(knots, degree, t)` (De Boor knot search), `domain(knots, degree)`.

---

## toolkit_easing

Easing, interpolation, and tweening — pure-math and dependency-light. Used by `toolkit_anim` but useful independently for UI transitions, camera animations, and any interpolated motion.

### Easing enum
`Linear`, `QuadIn`/`QuadOut`/`QuadInOut`, `CubicIn`/`CubicOut`/`CubicInOut`, `SineIn`/`SineOut`/`SineInOut`, `ElasticIn`/`ElasticOut`/`ElasticInOut`, `BounceOut`, `Spring( stiffness, damping )`. `ease(t)` returns the eased 0..1.

### Functions
- `ease(t, Easing)` → eased 0..1
- `lerp(a, b, t)` / `inverse_lerp(a, b, value)` / `remap(value, in_lo, in_hi, out_lo, out_hi)` — scalar helpers
- `Tween::new(duration, easing)` — time-driven, yields progress via `update(dt)`. `Repeat::Once`/`Repeat::Loop`/`Repeat::PingPong`.

---

## toolkit_noise

Coherent noise functions for procedural content. All noise is seeded and deterministic — same seed = same output across platforms.

### Noise
`new(seed)` → deterministic Perlin/simplex/value noise. Methods: `noise2(x, y)`, `noise3(x, y, z)`, `simplex2(x, y)`, `simplex3(x, y, z)`, `value2(x, y)`, `value3(x, y, z)`.

### Fbm / NoiseKind
`Fbm::new(kind)` layers a base noise into fractal detail. Methods: `sample2(&noise, x, y)` / `sample3(&noise, x, y, z)`. Config: `octaves`, `lacunarity`, `persistence`, `ridged` (boolean — creates mountain-like terrain). `NoiseKind::{Perlin, Simplex, Value}`.

### Worley
`worley2(x, y)`, `worley2_f2(x, y)`, `worley3(x, y, z)` — cellular (Voronoi) noise. Returns distance to nearest feature point. `_f2` = difference between nearest and second-nearest (bubble/vein patterns).

---

## toolkit_rng

Deterministic seeded random numbers. PCG32 generator: small, fast, reproducible across platforms for a given seed — the property procedural generation depends on.

### Rng
`seed_from_u64(seed)` / `from_u64_pair(seed, stream)`. Methods: `next_u32()`, `next_f32()`, `next_f64()`, `range_f32(lo, hi)`, `range_i32(lo, hi)`, `shuffle(slice)`, `pick(&[T])`.

### Geometric sampling
`unit_vec3()`, `unit_vec2()`, `inside_sphere()`, `inside_disk()`, `hemisphere(normal)`, `on_sphere()`, `on_disk()`.

### Poisson-disk
`poisson_disk_2d(width, height, radius, rng, max_attempts)` → `Vec<Vec2>` blue-noise point set. Uses the standard O(N) dart-throwing algorithm with a background grid.

---

## toolkit_sdf

Signed distance fields (SDF): implicit modeling with smooth booleans. Build a shape as an `Sdf` tree of primitives and CSG combinators, then `polygonize` it into a `toolkit_geometry::Mesh` with surface nets.

### Primitives
`Sphere { radius }`, `BoxSdf { half_extents }`, `Cylinder { radius, height }`, `Capsule { a, b, radius }`, `Plane { normal, d }`, `Torus { major, minor }`. Free functions: `sd_sphere`, `sd_box`, `sd_capsule`, `sd_cylinder`, `sd_plane`, `sd_round_box`, `sd_torus`, `sdf_normal(p, sdf)`.

### CSG combinators
`union(a, b)`, `intersection(a, b)`, `subtraction(a, b)`, `smooth_union(a, b, k)`, `smooth_intersection(a, b, k)`, `smooth_subtraction(a, b, k)`. Transform combinators: `translate(sdf, offset)`, `scale(sdf, factor)`. The `Sdf` trait gives all combinators as methods.

### Polygonize
`polygonize(sdf, &bounds, resolution)` → `Mesh`. Uses surface nets (dual contouring variant) for non-manifold-capable meshing.

---

## toolkit_spatial

Spatial acceleration structures for neighbour and range queries.

### SpatialHashGrid
Dynamic — insert/clear per frame. Best when points move and queries use a consistent radius (particles, broad-phase). `new(cell_size)`, `insert(id, pos)`, `query(pos, radius)` → neighbor ids, `clear()`.

### KdTree
Static — build once. Best for nearest-neighbour and radius queries over a fixed set (point clouds, sampling). `build(&[Vec3])`, `nearest(point)` → `Option<usize>`, `k_nearest(point, k)` → `Vec<(usize, f32)>`, `radius(point, radius)` → `Vec<(usize, f32)>`.

### Octree
Hierarchical box/range queries over uneven distributions. `new(bounds, max_depth, max_elements)`, `insert(aabb, id)`, `query(&aabb)` → ids, `remove(id)`.

---

## toolkit_units

Length unit system for CAD precision. `Length` stores metres canonically so arithmetic (`Add`/`Sub`/`Mul<f64>`/`Div`) is unit-agnostic; convert at the edges with `Length::new(value, unit)` / `in_unit()` / `format()`. `LengthUnit` (mm/cm/m/km/in/ft/yd/mi) knows its `meters_per_unit()` and `abbreviation()`. `UnitSystem { display, precision }` formats and `parse()`s user input (`"25.4 mm"`, `"3ft"`, or a bare number in the display unit), with `metric()` / `imperial()` presets.

---

## toolkit_color

Colour spaces over `toolkit_core::LinearRgba`. `Hsv` (hue rotation, picker-friendly) and `Oklab` (perceptually uniform, natural mixing) both round-trip via `from_linear()`/`to_linear()`. `Gradient` holds sorted `ColorStop`s, samples in `InterpolationSpace::{LinearRgb, Oklab}` via `sample(t)`, and emits a `ramp(count)`. `Palette` is a named colour set with `get_wrapping()` and `distinct_hues(count, sat, val)` for category colours.

---

## toolkit_intersect

Bounding-shape queries extending geometry's ray-only set. Shapes: `Plane` (`signed_distance`), `Sphere`, `Segment`, `Capsule`. Closest-point: `closest_point_on_segment/_aabb/_plane`, `closest_points_between_segments`. Overlap booleans: `sphere_sphere`, `sphere_aabb`, `sphere_plane`, `segment_sphere`(`_gap`), `capsule_capsule`, `aabb_plane_side`. `Frustum::from_view_projection()` (Gribb–Hartmann, `[0,1]` clip depth) with `intersects_sphere`/`intersects_aabb` (positive-vertex cull) and `contains_sphere`.

---

## toolkit_image

CPU RGBA8 image buffer for texture work and baking. `Image` stores packed sRGB bytes: `pixel`/`set_pixel`, `linear_at`/`set_linear` (via `LinearRgba`), `fill`. `sample_bilinear(u, v)` and `resize(w, h)` filter in linear space; `blit(src, ox, oy)` clips to bounds. PNG I/O: `encode_png`/`decode_png` (decode normalises palette/grayscale/16-bit/RGB → RGBA8) and `load_png`/`save_png`. Backed by the `png` crate.

---

## toolkit_project

The "open / save my work" bundle. `Project` holds a `Scene` plus the assets its nodes reference — meshes (UVs ride along in their vertices), PBR `materials`, and embedded `textures` — stored as `(id, value)` lists so they serialize to both JSON and binary. `add_mesh`/`add_material`/`add_texture` allocate ids; `insert_*` upserts by id; `mesh`/`material`/`texture` look up. `to_json`/`to_binary` (+ `save_*`/`load_*` files) round-trip the whole bundle; `validate()` reports dangling asset references. `ProjectMetadata` carries name, `UnitSystem`, and generator string.

---

## toolkit_skeleton

Joints, skin weights, posing, and linear-blend skinning. `Skeleton::new(joints)` links `Joint`s by parent index and caches inverse-bind matrices (parent chains are walked, so joint order is free). `Pose` supplies animated local transforms; `Pose::skinning_matrices(&skeleton)` yields the `global_pose * inverse_bind` palette. `apply_skin(mesh, skin, &palette)` deforms a mesh with a per-vertex `Skin` of `SkinWeights` (up to four influences, auto-normalised), blending matrices then transforming position and normal.

---

## toolkit_texture_bake

Project a mesh's surface into UV space and write maps. `rasterize_gbuffer(mesh, w, h)` walks each UV triangle at texel centres into a `GBuffer` of per-texel position + normal. From it: `bake_object_normal_map` (object-space, `n*0.5+0.5`), `bake_position_map` (remapped into a bounds box), and `bake_ambient_occlusion(mesh, gb, samples, max_distance, seed)` — hemisphere ray casting against a BVH, seeded per-texel for reproducibility. Outputs are raw-byte `Image`s (data, not sRGB colour).

---

## toolkit_convex

`convex_hull(points)` builds a `ConvexHull` (deduplicated vertices + outward triangles) via the incremental algorithm: seed a tetrahedron, then for each point remove the faces it sees and refan to the horizon, orienting against a fixed interior point. Provides `contains`, `face_normal`, and `to_mesh`. `gjk_distance(a, b)` / `hulls_intersect(a, b)` measure separation between two convex point sets through their Minkowski difference, with a full simplex sub-distance (point/segment/triangle/tetrahedron) reduction; `0.0` means overlap.

---

## toolkit_text

SDF text, rasterizer-agnostic. `coverage_to_sdf(w, h, coverage, spread)` converts a binary glyph mask to a signed distance field via 8SSEDT (crisp at any scale). `AtlasBuilder` shelf-packs glyph SDF `Image`s into one `FontAtlas`, recording each `Glyph`'s metrics (advance/offset/size) and atlas UV rect. `layout_text(atlas, text, options)` positions a string into `PositionedGlyph` quads with `\n` breaks and greedy word wrap (`LayoutOptions` carries scale, max width, line gap); returns a `TextLayout` with bounds.

---

## toolkit_lsystem

L-systems: `LSystem::new(axiom).rule(pred, succ)` defines rewriting; `expand(iterations)` runs it deterministically, `expand_stochastic(iterations, rng)` chooses among `weighted_rule` alternatives reproducibly. `interpret(string, &TurtleConfig)` runs a 3D turtle (local axes: forward `+Y`, up `+Z`, left `+X`) — `F`/`f` draw/move, `+-&^\/` rotate, `|` reverse, `[`/`]` branch — emitting `Segment`s.

---

## toolkit_wfc

Tiled wave-function collapse. `WfcModel` holds tiles (with weights) and symmetric `allow(tile, Dir, neighbor)` adjacency rules (`Dir::{Right,Left,Up,Down}`). `solve(model, width, height, rng)` repeatedly collapses the lowest-entropy cell to a weighted tile and propagates constraints, returning a row-major `WfcGrid` of tile ids — deterministic per seed, `None` on contradiction.
