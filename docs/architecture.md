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

---

## Know Weaknesses & Future-Proofing

This list is here so future-you (or contributors) know what to watch for.
None of these are blockers — they're risks that grow with the toolkit.

### 1. No semver policy → dependency breaks silently

Today every crate has `version.workspace = true`. When you update the toolkit
mid-project, there's no version bump telling you something changed. If you
refactor `toolkit_topology::HalfEdgeMesh`, projects using `toolkit_meshedit`
compile-fail with no warning.

**Fix later:** Add a `CHANGELOG.md` per crate or a workspace-level changelog.
Start versioning independently once the APIs stabilise (1.0).

### 2. `toolkit_core` is an accretion risk

Everything depends on `toolkit_core`. Right now it's small (IDs, events,
commands, TileMap, LinearRgba). If every new module puts shared types there,
it grows into a 5000-line monolith that every compilation rebuilds.

**Fix later:** Split off independent clusters:
- `toolkit_ids` (define_id! macro + atomic counter)
- `toolkit_color` stays separate (it already is)
- `toolkit_commands` (DocumentCommand, RenderCommand dispatcher)

### 3. No serialisation versioning → old files may not load

Types derive `Serialize`/`Deserialize`, but there's no schema version or
migration pass. A `Project` saved today with `toolkit_project` might not
deserialise after a struct field rename.

**Fix later:** Add a `version: u32` field to `ProjectMetadata`. Write a
`migrate_v1_to_v2()` function. Tag `#[serde(deny_unknown_fields)]` to
catch stale files early.

### 4. Heavy deps pulled even when unused

`toolkit_render` depends on `wgpu`. If your headless mesh processor only
needs `Camera::math` for ray generation, you still compile wgpu. Same
for `toolkit_ui` (egui).

**Fix later:** Split render into:
- `toolkit_render_core` — Camera, math, uniforms (tiny, no GPU dep)
- `toolkit_render_wgpu` — GpuContext, pipelines, TextureCache

Or use feature flags to make wgpu/egui optional.

### 5. No doc-test CI → docs drift from reality

Every code snippet in `module-reference.md` and `building-apps.md` is hand-
written. If a module's API changes, the doc snippet silently becomes wrong.
There's no CI step that runs `cargo test --doc` across the workspace.

**Fix later:** Add a CI workflow:
```yaml
# .github/workflows/ci.yml
- run: cargo test --workspace --doc
```
Or run it manually before committing:
```bash
cargo test --workspace --doc 2>&1 | grep "FAILED"
```

### 6. No benchmark suite → optimisations are guesswork

When you port an app optimisation back into a toolkit module ("I found a faster
way to compute this"), there's no baseline to compare against. You can't tell
if the change made things faster or slower.

**Fix later:** Add a `benches/` dir to crates with hot loops:
- `toolkit_geometry` — BVH build + intersect throughput
- `toolkit_solver` — CG convergence rate for known matrices
- `toolkit_remesh` — decimation throughput
- `toolkit_voxelize` — SDF generation time

### 7. No integration tests across crate boundaries

Each crate tests itself (`#[cfg(test)] mod tests` per file). But there's no
test that chains `geometry → topology → uv → texture_bake` and checks the
output matches an expected hash. Cross-crate integration bugs only show up
when you build an app.

**Fix later:** Add an `tests/` directory at workspace root with integration
tests that exercise common pipelines:
```rust
// tests/pipeline_uv_bake.rs
use toolkit_geometry::Mesh;
use toolkit_topology::HalfEdgeMesh;
use toolkit_uv::unwrap_charts;
// ... assert output is reasonable
```

### 8. Module granularity needs periodic review

Some crates are tiny (e.g. `toolkit_easing` is ~200 lines, `toolkit_select`
is ~300 lines). That's fine for separation of concerns — but if you end up
with 80 small crates, `cargo build` overhead (resolution, metadata) grows.

**Fix later:** If crate count exceeds ~70, merge closely related crates
behind feature flags:
- `toolkit_proc = { features = ["noise", "rng", "sdf", "wfc"] }`
- `toolkit_modeling = { features = ["meshedit", "select", "polyline"] }`

### 9. No `toolkit-prelude` for common imports

Every app file starts with a wall of `use` statements:
```rust
use toolkit_core::*;
use toolkit_geometry::*;
use toolkit_topology::*;
use toolkit_scene::*;
```

**Fix later:** Add a `toolkit_prelude` crate that re-exports the most common
types from every module. Apps add one import:
```rust
use toolkit_prelude::*;
```

### 10. Module generality audit — which crates carry app-specific assumptions

Every crate was designed with a "will this work in multiple apps?" test.
Most pass. A few have implicit assumptions from the original app they
served. Knowing these lets you decide when to use the crate as-is vs.
fork/extend.

| Crate | Verdict | Why |
|-------|---------|-----|
| `toolkit_core` | ✅ **General** | IDs, events, commands, color math — no app assumptions |
| `toolkit_input` | ✅ **General** | Stroke stabilizer works for any pointer-driven app |
| `toolkit_state` | ⚠️ **Paint-app bias** | `LayerKind::Paint / Fill / Folder / Mask / Adjustment` assume a 2D compositing app. The `HistoryStack` undo/redo is fully general, but the layer tree carries art-app assumptions |
| `toolkit_render` | ✅ **General** | Camera, OrbitController, PBR material, FlyController — reusable in any 3D app |
| `toolkit_geometry` | ✅ **General** | Mesh, BVH, ray — universal 3D types |
| `toolkit_graph` | ✅ **General** | Node graph DAG — any procedural system |
| `toolkit_simulation` | ⚠️ **Domain-specific** | `FluidSim` (2D stable fluids) and `ErosionSim` (hydraulic) are correct but narrow — they serve terrain/fluid editors specifically, not general simulation |
| `toolkit_ui` | ⚠️ **Default-layout bias** | The `WorkspaceLayout` defaults (3D Viewport, 2D Canvas, Layers, Properties, Color Picker) mirror a specific app. The `ToolkitTheme` and `PropertyGrid` widgets are fully general |
| `toolkit_scene` | ✅ **General** | Transform hierarchy, lights, selection — universal 3D scene graph |
| `toolkit_topology` | ✅ **General** | Half-edge mesh — standard geometry processing |
| `toolkit_uv` | ✅ **General** | LSCM unwrap, atlas packing — any 3D app needing UVs |
| `toolkit_gizmo` | ✅ **General** | Translate/rotate/scale gizmo — any 3D manipulator |
| `toolkit_canvas` | ✅ **General** | 2D pan/zoom/grid — any 2D viewport |
| `toolkit_assets` | ✅ **General** | OBJ/glTF — universal 3D formats |
| `toolkit_ai_bridge` | ⚠️ **Adapter bias** | The `AiProvider` trait and `McpServer` are fully general. But the adapter implementations (`DocumentBridge`, `CameraBridge`, `SceneBridge`) assume an editor-wrapping pattern. Rolling a game or a headless processor needs new adapters |
| `toolkit_attributes` | ✅ **General** | Columnar attribute channels — geometry/sim/particle data storage |
| `toolkit_volume` | ✅ **General** | Dense 3D grid — fluids, SDF baking, voxel sculpt |
| `toolkit_field` | ✅ **General** | `Field` trait + combinators — pure math composition |
| `toolkit_solver` | ✅ **General** | CG, Gauss-Seidel, Jacobi — any linear system |
| `toolkit_brush` | ✅ **General** | Falloff profiles + dab spacing — shared by sculpt, paint, terrain, weight |
| `toolkit_meshedit` | ✅ **General** | Extrude/inset/bevel/bridge — standard modeling ops |
| `toolkit_select` | ✅ **General** | Weighted selection + adjacency — any element-picking |
| `toolkit_polyline` | ✅ **General** | Resample / smooth / simplify — 2D or 3D paths |
| `toolkit_triangulate` | ✅ **General** | Ear-clip + Delaunay — any 2D poly fill |
| `toolkit_surfacing` | ✅ **General** | Extrude / loft / revolve / sweep — CAD basics |
| `toolkit_voxelize` | ✅ **General** | Mesh→SDF volume — bridge between surface and volumetric |
| `toolkit_remesh` | ✅ **General** | QEM decimate + cluster remesh — LOD/cleanup |
| `toolkit_meshops` | ✅ **General** | Weld / normals / flip / merge / smooth — utilities |
| `toolkit_anim` | ✅ **General** | Keyframe tracks + scene transform drive |
| `toolkit_curves` | ✅ **General** | Bezier, B-spline, NURBS — CAD standard |
| `toolkit_easing` | ✅ **General** | Pure-math easing + tweening — universally reusable |
| `toolkit_noise` | ✅ **General** | Perlin/Simplex/Worley/FBM — pure functions |
| `toolkit_rng` | ✅ **General** | PCG32 + geometric distributions — universally reusable |
| `toolkit_sdf` | ✅ **General** | Sphere/Box/CSG → Mesh — implicit modeling |
| `toolkit_spatial` | ✅ **General** | Kd-tree / octree / hash grid — spatial queries |
| `toolkit_units` | ✅ **General** | Length units — CAD / any measurement app |
| `toolkit_color` | ✅ **General** | HSV/Oklab/gradients — universal color math |
| `toolkit_intersect` | ✅ **General** | Frustum/plane/capsule intersection — any spatial app |
| `toolkit_image` | ✅ **General** | RGBA buffer + PNG — texture work, baking, screenshots |
| `toolkit_project` | ✅ **General** | Scene+mesh+material bundle — any 3D app with save/load |
| `toolkit_skeleton` | ✅ **General** | Joint hierarchy + LBS skinning — character rigging |
| `toolkit_texture_bake` | ✅ **General** | UV raster → AO/normal maps — any baking pipeline |
| `toolkit_convex` | ✅ **General** | QuickHull + GJK — collision, bounds, physics |
| `toolkit_text` | ✅ **General** | SDF font atlas + layout — UI, labels, measurements |
| `toolkit_lsystem` | ✅ **General** | Grammar + turtle — procedural generation |
| `toolkit_wfc` | ✅ **General** | Tile adjacency → grid — procedural generation |

**Summary:** 38 of 46 crates are general-purpose. The 8 with caveats
(`toolkit_state`, `toolkit_simulation`, `toolkit_ui`, `toolkit_ai_bridge`
adapters, plus the 4 not-yet-implemented) carry assumptions from the
original editor they served. This is not a problem — it's an honest
inventory. When you build a new app in a different domain (e.g. game,
CAD, data viz), you'll know which crates to vendor and which to extend.

### Summary

The toolkit is **well-designed for the growing-reusable-library model**. The
risks above are maturity risks, not design flaws — they're what happens when
a library is used by one person across multiple apps without formal release
management. Most are cheap to fix when the time comes.
