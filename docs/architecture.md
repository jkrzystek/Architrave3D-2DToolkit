# 3dRustToolkit Architecture

## Overview

The toolkit is a Cargo workspace of 42 independent, composable crates. Each crate handles one concern and can be used standalone or combined with others to build 2D/3D editor applications.

```
app-3d-toolkit/
├── Cargo.toml              (workspace root)
├── crates/
│   ├── toolkit_core        Foundation (IDs, events, commands, color, tile map)
│   │
│   ├── toolkit_attributes  Typed attribute channels (sculpt masks, weights, UV sets)
│   ├── toolkit_volume      Dense 3D scalar/vector grids with trilinear sampling
│   ├── toolkit_field       Thin "value at a point" trait + combinators
│   ├── toolkit_solver      Sparse matrices + iterative solvers (CG, Gauss-Seidel, Jacobi)
│   │
│   ├── toolkit_input       Input telemetry, stroke stabilizer, stroke recording
│   ├── toolkit_state       Document model, layer tree, blend modes, undo/redo
│   ├── toolkit_render      GPU context, textures, camera, PBR, navigation
│   ├── toolkit_ui          egui viewport, panels, theme, widgets
│   │
│   ├── toolkit_geometry    Vertex, mesh, AABB, BVH, ray intersection
│   ├── toolkit_topology    Half-edge mesh, subdivision, topology editing
│   ├── toolkit_uv          UV unwrapping (LSCM), charts, atlas packing
│   ├── toolkit_gizmo       Transform gizmo logic (hit-test + drag deltas)
│   ├── toolkit_meshedit    Poly-modeling ops (extrude/inset/bevel/bridge/loop-cut)
│   ├── toolkit_select      Weighted (soft) selection sets with grow/shrink
│   ├── toolkit_polyline    2D/3D polyline ops (resample, smooth, simplify, offset)
│   ├── toolkit_canvas      2D editor view (pan/zoom, grid, selection)
│   │
│   ├── toolkit_brush       Stroke + falloff engine (sculpt/paint/weight brush)
│   ├── toolkit_triangulate 2D polygon triangulation (ear-clip, constrained Delaunay)
│   ├── toolkit_surfacing   Generate meshes from profiles (extrude, loft, revolve, sweep)
│   ├── toolkit_voxelize    Mesh → volume (signed distance + solid occupancy)
│   ├── toolkit_remesh      QEM decimation + cluster remeshing
│   ├── toolkit_meshops     Mesh utilities (weld, normals, flip, merge, Laplacian smooth)
│   │
│   ├── toolkit_scene       Transform hierarchy, nodes, lights, selection
│   ├── toolkit_assets      OBJ / glTF import & export
│   ├── toolkit_ai_bridge   AI/LLM integration (MCP server, module adapters)
│   │
│   ├── toolkit_anim        Keyframe animation (Track, TransformAnimation, Player)
│   ├── toolkit_curves      Parametric curves/surfaces (Bezier, B-spline, NURBS)
│   ├── toolkit_easing      Easing curves + tweening (cubic, elastic, spring)
│   ├── toolkit_noise       Perlin/Simplex/Worley noise + FBM fractals
│   ├── toolkit_rng         Deterministic seeded PRNG + distributions
│   ├── toolkit_sdf         Signed-distance primitives + CSG + surface nets
│   ├── toolkit_spatial     Spatial hash grid + kd-tree + octree
│   │
│   ├── toolkit_simulation  Fluid simulation + hydraulic erosion on 2D grids
│   ├── toolkit_graph       Procedural node graph (DAG), evaluation engine
│   │
│   ├── toolkit_units       Length unit system (mm/cm/m/in/ft) for CAD precision
│   ├── toolkit_color       HSV/Oklab colorspaces, gradients, palettes
│   ├── toolkit_intersect   Frustum culling, sphere/plane/capsule intersection
│   ├── toolkit_image       CPU RGBA8 image buffer, PNG I/O
│   ├── toolkit_project     Open/save project bundle (scene + assets + materials)
│   │
│   ├── toolkit_skeleton    Joints, skin weights, pose, LBS deformation
│   ├── toolkit_texture_bake AO/normal/position map baking from UV charts
│   ├── toolkit_convex      Convex hull (QuickHull) + GJK distance
│   ├── toolkit_text        SDF font atlas + text layout
│   ├── toolkit_lsystem     L-system grammar + 3D turtle interpretation
│   └── toolkit_wfc         Tiled wave-function collapse
├── docs/                   This documentation
└── toolkitDocs/            Original design documents
```

`toolkit_render` also provides PBR materials (`PbrMaterial`,
`MaterialUniforms`, a reference Cook-Torrance WGSL shader) and camera
navigation (`FlyController`, framing helpers) alongside the GPU layer.

## Dependency Graph

```
toolkit_core  (foundation, no internal deps)
     │
     ├── toolkit_attributes    (glam, serde)
     ├── toolkit_volume        (glam, serde)
     ├── toolkit_field         (glam — no deps beyond core)
     ├── toolkit_solver        (glam — no deps beyond core)
     ├── toolkit_input         (glam)
     ├── toolkit_state         (serde)
     ├── toolkit_render        (wgpu, bytemuck, glam)
     ├── toolkit_geometry      (glam, bytemuck)
     ├── toolkit_meshops       (geometry — utility on top)
     ├── toolkit_graph         (petgraph, serde)
     ├── toolkit_simulation    (glam)
     ├── toolkit_ui            (egui)
     ├── toolkit_scene         (glam, generational-arena)
     ├── toolkit_topology      (geometry)
     ├── toolkit_uv            (glam — solver embedded)
     ├── toolkit_gizmo         (geometry)
     ├── toolkit_select        (glam)
     ├── toolkit_meshedit      (topology, geometry)
     ├── toolkit_triangulate   (glam)
     ├── toolkit_surfacing     (geometry)
     ├── toolkit_voxelize      (geometry)
     ├── toolkit_remesh        (geometry)
     ├── toolkit_polyline      (glam)
     ├── toolkit_canvas        (glam)
     ├── toolkit_brush         (glam)
     ├── toolkit_anim          (easing, scene)
     ├── toolkit_curves        (geometry)
     ├── toolkit_easing        (glam — pure math)
     ├── toolkit_noise         (glam — pure functions)
     ├── toolkit_rng           (glam)
     ├── toolkit_sdf           (geometry)
     ├── toolkit_spatial       (glam)
     ├── toolkit_assets        (scene, geometry)
     ├── toolkit_ai_bridge     (all modules, via feature flags)
     ├── toolkit_units         (serde)
     ├── toolkit_color         (core)
     ├── toolkit_intersect     (glam)
     ├── toolkit_image         (png crate, core)
     ├── toolkit_project       (scene, geometry, units)
     ├── toolkit_skeleton      (geometry)
     ├── toolkit_texture_bake  (geometry, image, uv, topology)
     ├── toolkit_convex        (glam)
     ├── toolkit_text          (image)
     ├── toolkit_lsystem       (glam)
     └── toolkit_wfc           (rng)
```

All crates depend on `toolkit_core` for shared types (IDs, events, commands).
No other inter-crate dependencies exist beyond what's listed above — each
crate pulls in only what it needs.

`toolkit_ai_bridge` is the exception: it depends on all modules (via optional
feature flags) to provide AI/LLM access adapters.

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

| Tier | Crate | Purpose | Key Types |
|------|-------|---------|-----------|
| **Core** | `toolkit_core` | Shared foundation | `LayerId`, `ViewportInputEvent`, `TileMap<T>`, `LinearRgba` |
| | `toolkit_input` | Input processing | `InputBuffer`, `StrokeStabilizer`, `Stroke` |
| | `toolkit_state` | Document model | `Document`, `Layer`, `HistoryStack`, `blend()` |
| | `toolkit_render` | GPU + PBR + navigation | `GpuContext`, `Camera`, `OrbitController`, `PbrMaterial`, `FlyController` |
| | `toolkit_ui` | Interface (egui) | `ViewportPanel`, `WorkspaceLayout`, `ToolkitTheme`, `PropertyGrid` |
| **Substrates** | `toolkit_attributes` | Typed attribute channels | `AttributeStore`, `AttributeSet`, `Attribute`, `Domain` |
| | `toolkit_volume` | Dense 3D grids | `Volume<T>`, `VolumeSample` |
| | `toolkit_field` | Composable field trait | `Field`, `VectorField`, `FieldExt` combinators |
| | `toolkit_solver` | Sparse linear solvers | `SparseMatrix`, `solve_cg`, `solve_least_squares` |
| **3D Pipeline** | `toolkit_geometry` | Mesh & spatial queries | `Vertex`, `Mesh`, `Aabb`, `Bvh`, `Ray` |
| | `toolkit_topology` | Half-edge mesh editing | `HalfEdgeMesh`, `MeshSelection`, `catmull_clark` |
| | `toolkit_uv` | UV unwrapping | `unwrap_lscm`, `Chart`, `pack_charts`, projections |
| | `toolkit_gizmo` | Transform gizmo | `Gizmo`, `GizmoMode`, `GizmoDelta` |
| | `toolkit_scene` | Scene graph | `Scene`, `SceneNode`, `Transform`, `Light`, `Selection` |
| | `toolkit_assets` | Asset I/O | `ImportedScene`, `import_obj_str`, `import_gltf_slice` |
| | `toolkit_ai_bridge` | AI integration | `AiProvider`, `BridgeRegistry`, `McpServer` |
| **Modeling** | `toolkit_meshedit` | Poly-modeling ops | `EditMesh`, `extrude`, `inset`, `bevel`, `loop_cut` |
| | `toolkit_select` | Weighted selection | `Selection`, `Adjacency`, grow/shrink, soft falloff |
| | `toolkit_polyline` | Polyline operations | `resample`, `simplify`, `smooth_chaikin`, `offset_2d` |
| | `toolkit_brush` | Stroke + falloff engine | `Brush`, `Falloff`, dab spacing |
| | `toolkit_canvas` | 2D editor view | `CanvasView`, `adaptive_step`, `SelectionDrag` |
| **Gen/Surface** | `toolkit_triangulate` | 2D polygon triangulation | `triangulate`, `triangulate_with_holes`, `Triangulation` |
| | `toolkit_surfacing` | Profile → surface | `extrude`, `loft`, `revolve`, `sweep` |
| | `toolkit_voxelize` | Mesh → volume | `signed_distance_field`, `solid`, `surface_shell` |
| | `toolkit_remesh` | Simplify + remesh | `decimate_to`, `decimate_ratio`, `cluster_remesh` |
| | `toolkit_meshops` | Mesh utilities | `weld_vertices`, `recompute_normals`, `laplacian_smooth` |
| **Animation** | `toolkit_anim` | Keyframe animation | `Track<T>`, `TransformAnimation`, `AnimationPlayer` |
| | `toolkit_curves` | Parametric curves | `Bezier`, `BSplineCurve`, `NurbsCurve`, `NurbsSurface` |
| | `toolkit_easing` | Easing + tweening | `Easing`, `Tween`, `lerp` |
| **Procedural** | `toolkit_noise` | Coherent noise | `Noise`, `Fbm`, `worley2`, `NoiseKind` |
| | `toolkit_rng` | Seeded random | `Rng`, `poisson_disk_2d`, geometric sampling |
| | `toolkit_sdf` | SDF modeling | `Sphere`, `BoxSdf`, `smooth_union`, `polygonize` |
| | `toolkit_spatial` | Spatial queries | `KdTree`, `SpatialHashGrid`, `Octree` |
| | `toolkit_graph` | Node graph (DAG) | `NodeGraph`, `NodeTemplate`, `NodeRegistry`, `evaluate_graph()` |
| | `toolkit_simulation` | 2D physics | `FluidSim`, `ErosionSim`, `Grid2D<T>` |
| **Utilities** | `toolkit_units` | Length units | `Length`, `LengthUnit`, `UnitSystem` |
| | `toolkit_color` | Color spaces | `Hsv`, `Oklab`, `Gradient`, `Palette` |
| | `toolkit_intersect` | Intersection tests | `Frustum`, `Capsule`, `sphere_aabb`, GJK in convex |
| | `toolkit_image` | Image buffer | `Image`, `sample_bilinear`, `encode_png`, `decode_png` |
| | `toolkit_project` | Project bundle | `Project`, `ProjectMetadata`, `to_json`, `to_binary` |
| **Advanced** | `toolkit_skeleton` | Skinning | `Skeleton`, `Pose`, `Skin`, `apply_skin` |
| | `toolkit_texture_bake` | Map baking | `rasterize_gbuffer`, `bake_ambient_occlusion` |
| | `toolkit_convex` | Convex hull + GJK | `ConvexHull`, `gjk_distance`, `hulls_intersect` |
| | `toolkit_text` | SDF text | `FontAtlas`, `Glyph`, `layout_text`, `TextLayout` |
| | `toolkit_lsystem` | L-systems | `LSystem`, `interpret`, `TurtleConfig` |
| | `toolkit_wfc` | Wave-function collapse | `WfcModel`, `solve`, `WfcGrid` |

### Dependency tiers (logical, not strict)

```
toolkit_core ── host to all

Substrates (Tier A):
  toolkit_attributes  toolkit_volume  toolkit_field  toolkit_solver
  └── all lean on core + glam only

3D Pipeline (original + extensions):
  geometry ── topology ── uv
  geometry ── gizmo
  geometry ── meshedit
  assets ── scene ── ai_bridge

Modeling (Tier B):
  brush      (standalone stroke/falloff)
  select     (weighted selection + adjacency)
  polyline   (2D/3D polyline ops)
  meshedit   (topology + geometry)

Surface/Gen (Tier C):
  triangulate ── surfacing ── voxelize ── remesh
  meshops    (utility over geometry)

Procedural / Animation:
  easing ── anim ── scene
  curves ── geometry
  noise ── rng ── sdf ── spatial
  graph ── simulation

Utilities:
  units  color  intersect  image  project

Advanced:
  skeleton  texture_bake  convex  text
  lsystem ── wfc
```

Each crate depends only on `toolkit_core` plus specific domain crates — nothing
else. This keeps compilation incremental and lets you pull in only what you need.
