# Building Apps with the Toolkit

This guide shows how to use toolkit crates to build a complete application.

## Choosing Your Crates

Pick only the crates you need. The toolkit is a la carte — every crate works standalone.

### By app type

| Building... | Core crates you need | Optional extras |
|-------------|---------------------|-----------------|
| **3D model viewer** | `core`, `render`, `geometry`, `scene`, `assets`, `ui` | `intersect` (frustum culling), `scene` camera lighting |
| **3D modeling tool** | `core`, `render`, `geometry`, `scene`, `gizmo`, `ui`, `topology`, `meshedit`, `select` | `meshops`, `polyline`, `attributes` (sculpt masks), `remesh` |
| **UV editor** | `core`, `geometry`, `topology`, `uv`, `canvas`, `ui` | `image` (texture preview), `texture_bake` |
| **2D paint app** | `core`, `input`, `state`, `render`, `canvas`, `ui`, `brush` | `color`, `image`, `easing` (brush pressure curves) |
| **SDF / implicit modeler** | `core`, `geometry`, `render`, `sdf`, `ui` | `field`, `noise` (FBM-field SDF), `remesh`, `voxelize` |
| **Node-based generator** | `core`, `graph`, `geometry`, `render`, `ui` | `field`, `noise`, `rng`, `sdf`, `curves`, `wfc`, `lsystem` |
| **Terrain editor** | `core`, `state`, `render`, `geometry`, `simulation`, `ui`, `brush` | `noise` (heightmap gen), `spatial` (LOD culling), `remesh` |
| **CAD / procedural modeler** | `core`, `geometry`, `curves`, `surfacing`, `triangulate`, `render`, `ui` | `units`, `solver` (deformation), `meshops` |
| **VFX / particle sim** | `core`, `render`, `geometry`, `scene`, `simulation`, `volume`, `spatial` | `field` (force fields), `rng`, `noise` |
| **AI-accessible editor** | Any of the above + `ai_bridge` | Bridges each module: `adapter-*` feature flags |
| **Headless mesh processor** | `core`, `geometry`, `topology`, `assets`, `meshops`, `remesh` | `voxelize`, `sdf`, `convex`, `texture_bake` |
| **Animation rigging** | `core`, `geometry`, `scene`, `skeleton`, `anim`, `easing`, `render`, `ui` | `curves` (FK splines), `intersect` (IK ray casts) |

### By capability area

| You want to... | Add these crates |
|---------------|-----------------|
| Store arbitrary data on geometry (masks, weights, UV sets) | `attributes` |
| Sample/combine scalar fields (noise + SDF + volume) | `field`, `noise`, `sdf`, `volume` |
| Solve sparse linear systems (smoothing, deformation, fluids) | `solver` |
| Draw/paint/sculpt with pressure-sensitive brushes | `brush`, `input` |
| Edit polygonal meshes (extrude, bevel, loop-cut) | `topology`, `meshedit` |
| Select vertices/edges/faces with soft falloff | `select` |
| Manipulate polylines (resample, smooth, simplify, offset) | `polyline` |
| Triangulate 2D polygons with holes | `triangulate` |
| Generate surfaces from profiles (extrude, loft, revolve) | `surfacing` |
| Convert meshes to volumes and back | `voxelize`, `remesh`, `volume` |
| Clean up / weld / renormalize meshes | `meshops` |
| Animate scene transforms | `anim`, `easing` |
| Build parametric curves and surfaces (Bezier, NURBS) | `curves` |
| Generate procedural content (noise, SDF, L-system, WFC) | `noise`, `rng`, `sdf`, `lsystem`, `wfc` |
| Query spatial neighbors (kd-tree, octree, hash grid) | `spatial` |
| Add AI/LLM access to your app | `ai_bridge` |
| Save/load whole project bundles | `project`, `units` |
| Work with colors beyond RGB (HSV, Oklab, gradients) | `color` |
| Intersect / cull with frustums, capsules, spheres | `intersect` |
| Load/save PNG textures and image buffers | `image` |
| Rig and skin characters | `skeleton` |
| Bake AO/normal/position maps from UVs | `texture_bake` |
| Compute convex hulls and GJK distances | `convex` |
| Render text with SDF fonts | `text` |

See **`docs/3d-pipeline.md`** for a worked example chaining assets → topology
→ UV → scene → gizmo.

See **`docs/module-reference.md`** for the full API of every crate.

## Minimal App Setup

### Step 1: Create Your App Crate

```bash
cargo init my-3d-app
```

```toml
# my-3d-app/Cargo.toml
[dependencies]
toolkit_core = { path = "../app-3d-toolkit/crates/toolkit_core" }
toolkit_state = { path = "../app-3d-toolkit/crates/toolkit_state" }
toolkit_render = { path = "../app-3d-toolkit/crates/toolkit_render" }
toolkit_ui = { path = "../app-3d-toolkit/crates/toolkit_ui" }
# ... add others as needed
```

> **Tip**: Don't copy the whole workspace — add individual crate paths so Cargo
> only compiles what you actually use.

### Step 2: Initialize Core Systems

```rust
use toolkit_core::*;
use toolkit_state::Document;
use toolkit_render::camera::{Camera, OrbitController};
use toolkit_ui::{ToolkitTheme, WorkspaceLayout};

fn main() {
    // Document model
    let mut document = Document::new("My Project", 1920, 1080);
    document.add_layer("Background", LayerKind::Paint, None);
    document.add_layer("Foreground", LayerKind::Paint, None);

    // Camera
    let camera = Camera::perspective(
        glam::Vec3::new(0.0, 5.0, 10.0),
        glam::Vec3::ZERO,
        45.0,
        16.0 / 9.0,
    );
    let orbit = OrbitController::default();

    // UI theme
    let theme = ToolkitTheme::default(); // dark mode
    let layout = WorkspaceLayout::default();

    // Start your render/event loop here...
}
```

### Step 3: Handle Input Events

```rust
use toolkit_core::{ViewportInputEvent, PointerButton};
use toolkit_input::{StrokeStabilizer, StabilizerConfig, Stroke, StrokePoint};

let mut stabilizer = StrokeStabilizer::new(StabilizerConfig::default());
let mut current_stroke = Stroke::new();

// In your event loop:
fn handle_event(event: ViewportInputEvent) {
    match event {
        ViewportInputEvent::PointerPressed { position, button, .. } => {
            if button == PointerButton::Primary {
                stabilizer.reset(position);
                current_stroke = Stroke::new();
            }
        }
        ViewportInputEvent::PointerMoved { position, .. } => {
            if let Some(smoothed) = stabilizer.update(position, dt) {
                current_stroke.push_point(StrokePoint {
                    position: smoothed,
                    pressure: 1.0,
                    tilt: glam::Vec2::ZERO,
                    timestamp_ms: now,
                });
            }
        }
        _ => {}
    }
}
```

### Step 4: Use the Command Dispatcher

```rust
use toolkit_core::{DocumentCommand, RenderCommand, BlendMode};

// Create dispatcher
let dispatcher = ChannelDispatcher::new(256);

// Send commands from input handling
dispatcher.send_document(DocumentCommand::AddLayer {
    name: "New Layer".into(),
    kind: LayerKind::Paint,
    parent: None,
});

// Process commands in your update loop
while let Some(cmd) = dispatcher.try_recv_document() {
    match cmd {
        DocumentCommand::AddLayer { name, kind, parent } => {
            document.add_layer(name, kind, parent);
        }
        DocumentCommand::Undo => { /* ... */ }
        _ => {}
    }
}
```

### Step 5: Work with Meshes and Geometry

```rust
use toolkit_geometry::{Mesh, Bvh, Ray};

// Create meshes
let terrain = Mesh::plane(100.0, 100.0, 64);
let cube = Mesh::cube(2.0);
let sphere = Mesh::uv_sphere(1.0, 32, 16);

// Build BVH for raycasting
let bvh = Bvh::build(&terrain);

// Raycast on click
let ray = Ray::new(camera.position, click_direction);
if let Some(hit) = bvh.intersect(&ray, &terrain) {
    println!("Hit at {:?}, normal {:?}", hit.position, hit.normal);
}
```

### Step 6: Node Graph (Procedural)

```rust
use toolkit_graph::*;

let mut graph = NodeGraph::new();
let mut registry = NodeRegistry::new();
registry.register(FloatConstant);
registry.register(AddFloat);
registry.register(MultiplyFloat);

// Build a graph: Constant(5) + Constant(3) = 8
let c1 = graph.add_node("FloatConstant".into(), (0.0, 0.0));
let c2 = graph.add_node("FloatConstant".into(), (0.0, 100.0));
let add = graph.add_node("AddFloat".into(), (200.0, 50.0));

graph.set_input(c1, 0, NodeValue::Float(5.0));
graph.set_input(c2, 0, NodeValue::Float(3.0));

graph.connect(c1, 0, add, 0).unwrap();
graph.connect(c2, 0, add, 1).unwrap();

evaluate_graph(&mut graph, &registry).unwrap();
let result = graph.node(add).unwrap().cached_outputs.as_ref().unwrap();
// result[0] == NodeValue::Float(8.0)
```

### Step 7: Simulation

```rust
use toolkit_simulation::*;

// Fluid simulation
let mut fluid = FluidSim::new(128, 128, FluidConfig::default());
fluid.add_density(64, 64, 100.0);
fluid.add_velocity(64, 64, 5.0, 0.0);
fluid.step();

// Erosion simulation
let heightmap = Grid2D::new(256, 256, 0.5);
let mut erosion = ErosionSim::new(heightmap, ErosionConfig::default());
erosion.run(100); // run 100 iterations
let terrain = erosion.terrain(); // read result
```

## Building with the Toolbox (New Modules)

Here are quick-start patterns for the newer crates, organized by capability.

### Attributes — store arbitrary data on geometry

```rust
use toolkit_attributes::{AttributeStore, Domain, AttributeType};

let mut store = AttributeStore::new();
// Create a "Cd" (color) channel on the point domain with 3 elements.
let cd = store.create(Domain::Point, 3, "Cd", AttributeType::Color);
cd.set_vec4(0, glam::Vec4::new(1.0, 0.0, 0.0, 1.0)); // red first point

// Read it back
let cd = store.get(Domain::Point, "Cd").unwrap();
assert_eq!(cd.get_vec4(0).unwrap()[0], 1.0);
```

### Volume — 3D grids with trilinear sampling

```rust
use toolkit_volume::Volume;
use glam::Vec3;

// Build a scalar ramp along x
let vol = Volume::from_fn([4, 4, 4], Vec3::ZERO, Vec3::ONE, |[x, _, _]| x as f32);
let val = vol.sample(Vec3::new(0.5, 0.0, 0.0)); // ≈ 0.5

// Convert back to a mesh with marching
// (pair with toolkit_voxelize or toolkit_sdf::polygonize)
```

### Field — compose scalar/vector fields

```rust
use toolkit_field::{Field, FieldExt, Sphere};
use glam::Vec3;

// Union of two spheres with a shell
let a = Sphere { center: Vec3::new(-0.5, 0.0, 0.0), radius: 1.0 };
let b = Sphere { center: Vec3::new(0.5, 0.0, 0.0), radius: 1.0 };
let shape = a.min(b).map(|d| d - 0.1);
let inside = shape.sample(Vec3::ZERO) < 0.0; // true — overlaps
```

### Solver — sparse linear systems

```rust
use toolkit_solver::{SparseMatrix, solve_cg};

// Solve [[4,1],[1,3]] x = [1,2]
let mut a = SparseMatrix::new(2, 2);
a.push(0, 0, 4.0); a.push(0, 1, 1.0);
a.push(1, 0, 1.0); a.push(1, 1, 3.0);
let x = solve_cg(&a, &[1.0, 2.0], 50, 1e-10);
// x ≈ (0.0909, 0.6364)
```

### Brush — stroke stamping for sculpt/paint/weight

```rust
use toolkit_brush::{Brush, Falloff};
use glam::Vec3;

let mut brush = Brush::new(2.0, 1.0);
brush.falloff = Falloff::Smooth;
let stroke = vec![Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)];
let dabs = brush.dab_centers(&stroke);
let weight_at_center = brush.stroke_weight(&dabs, Vec3::ZERO); // > 0.0
```

### MeshEdit — poly modeling ops

```rust
use toolkit_meshedit::EditMesh;
use toolkit_topology::HalfEdgeMesh;
use toolkit_geometry::Mesh;

let cube = Mesh::cube(1.0);
let mut edit = EditMesh::from_halfedge(&HalfEdgeMesh::from_mesh(&cube));
edit.extrude_face(edit.face_count() - 1, 0.5);
let modified = edit.to_mesh("extruded");
```

### Select — weighted selection sets

```rust
use toolkit_select::{Selection, Adjacency};

// Select a vertex, grow one ring
let adj = Adjacency::from_pairs(4, &[(0, 1), (1, 2), (2, 3)]);
let grown = Selection::from_indices([1]).grow(&adj);
assert!(grown.contains(0) && grown.contains(2)); // neighbors filled
```

### Polyline — resample, smooth, simplify, offset

```rust
use toolkit_polyline::{resample, simplify, smooth_chaikin};
use glam::Vec2;

let stroke = vec![Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)];
let even = resample(&stroke, 5);      // 5 evenly-spaced points
let cleaned = simplify(&stroke, 0.01); // drops collinear points -> 2
```

### Triangulate — 2D polygons with holes

```rust
use toolkit_triangulate::triangulate_with_holes;
use glam::Vec2;

let outer = vec![Vec2::new(0.0,0.0), Vec2::new(4.0,0.0), Vec2::new(4.0,4.0), Vec2::new(0.0,4.0)];
let hole  = vec![Vec2::new(1.0,1.0), Vec2::new(1.0,3.0), Vec2::new(3.0,3.0), Vec2::new(3.0,1.0)];
let tri = triangulate_with_holes(&outer, &[hole]);
// tri.triangles covers area 12 (16 - 4)
```

### Surfacing — extrusions, lofts, revolves, sweeps

```rust
use toolkit_surfacing::{extrude, revolve, loft};
use glam::Vec2;

let square = vec![Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0),
                   Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)];
let box_mesh = extrude(&square, 2.0, true); // 12 triangles (closed box)

let cup_mesh = revolve(&[(1.0, 0.0), (1.0, 2.0), (2.0, 3.0)], 16);
```

### Voxelize — mesh to volume

```rust
use toolkit_voxelize::{signed_distance_field, VoxelizeConfig};
use toolkit_geometry::Mesh;

let sdf = signed_distance_field(&Mesh::cube(2.0),
    &VoxelizeConfig { resolution: 12, padding: 0.5 });
assert!(sdf.sample(glam::Vec3::ZERO) < 0.0); // inside cube
```

### Remesh — simplification

```rust
use toolkit_remesh::decimate_ratio;
use toolkit_geometry::Mesh;

let sphere = Mesh::uv_sphere(1.0, 24, 16);
let half = decimate_ratio(&sphere, 0.5);   // ~50% fewer tris
```

### Meshops — weld, normals, Laplacian smooth

```rust
use toolkit_geometry::Mesh;
use toolkit_meshops::{weld_vertices, recompute_normals, laplacian_smooth};

let cube = Mesh::cube(1.0);                     // 24 vertices
let welded = weld_vertices(&cube, 1e-4);         // 8 shared corners
let smooth = laplacian_smooth(&welded, 0.5, 5);  // relax positions
```

### Animation

```rust
use toolkit_anim::{TransformAnimation, AnimationPlayer};

let mut anim = TransformAnimation::new();
anim.translation.add(0.0, glam::Vec3::ZERO)
    .add(2.0, glam::Vec3::new(10.0, 0.0, 0.0));

let mut player = AnimationPlayer::new(anim.duration());
player.update(1.0); // 1s in
let t = anim.sample(player.time);
```

### Curves — Bézier, B-spline, NURBS

```rust
use toolkit_curves::BSplineCurve;
use glam::Vec3;

let curve = BSplineCurve::new(
    vec![Vec3::ZERO, Vec3::new(1.0, 2.0, 0.0), Vec3::new(3.0, 2.0, 0.0), Vec3::new(4.0, 0.0, 0.0)],
    3,
);
let polyline = curve.tessellate(32); // 33 points
```

### Noise — Perlin / Simplex / Worley / FBM

```rust
use toolkit_noise::{Noise, Fbm, NoiseKind};

let noise = Noise::new(1234);
let fbm = Fbm::new(NoiseKind::Simplex);
let height = fbm.sample2(&noise, 3.2, 1.7);
```

### RNG — deterministic random + Poisson-disk

```rust
use toolkit_rng::Rng;

let mut rng = Rng::seed_from_u64(42);
let val: f32 = rng.range_f32(-1.0, 1.0);
let dir = rng.unit_vec3();               // uniform on sphere
let pts = toolkit_rng::poisson_disk_2d(10.0, 10.0, 0.5, &mut rng, 30);
```

### SDF — implicit modeling

```rust
use toolkit_sdf::{Sphere, BoxSdf, smooth_union, polygonize};
use toolkit_geometry::Aabb;

let shape = smooth_union(
    Box::new(BoxSdf { half_extents: glam::Vec3::splat(0.8) }),
    Box::new(Sphere { radius: 1.0 }),
    0.3,
);
let mesh = polygonize(shape.as_ref(),
    &Aabb::new(glam::Vec3::splat(-2.0), glam::Vec3::splat(2.0)), 24);
```

### Spatial — kd-tree / octree / hash grid

```rust
use toolkit_spatial::KdTree;

let pts = vec![glam::Vec3::new(3.0, 0.0, 0.0), glam::Vec3::new(0.1, 0.0, 0.0)];
let tree = KdTree::build(&pts);
assert_eq!(tree.nearest(glam::Vec3::ZERO), Some(1));
```

### Easing

```rust
use toolkit_easing::{Tween, Easing, lerp};

let mut t = Tween::new(2.0, Easing::CubicInOut);
let p = t.update(1.0);                    // 0.5s in, eased
let value = lerp(0.0, 100.0, p);          // interpolate
```

### Skeleton — character rigging

```rust
use toolkit_skeleton::{Skeleton, Pose, Skin, apply_skin};
use toolkit_geometry::Mesh;

let skeleton = Skeleton::new(vec![
    /* parent_idx: None, 0, 0, ... */
]);
let pose = Pose::default(); // identity
let palette = pose.skinning_matrices(&skeleton);

let skin = Skin { weights: vec![/* per-vertex SkinWeights */] };
let deformed = apply_skin(&mesh, &skin, &palette);
```

### Texture Bake — AO / normal maps

```rust
use toolkit_texture_bake::{rasterize_gbuffer, bake_ambient_occlusion};
use toolkit_geometry::Mesh;
use toolkit_topology::HalfEdgeMesh;

let he = HalfEdgeMesh::from_mesh(&mesh);
// Assumes mesh has UVs
let gbuf = rasterize_gbuffer(&he.to_mesh("uvd"), 512, 512);
let ao = bake_ambient_occlusion(&mesh, &gbuf, 64, 2.0, 42);
// ao is a toolkit_image::Image (RGBA8, use .pixel() to read)
```

### Convex hull + GJK

```rust
use toolkit_convex::{convex_hull, gjk_distance};

let pts = vec![glam::Vec3::new(1.0, 0.0, 0.0), /* ... */];
let hull = convex_hull(&pts);
let mesh = hull.to_mesh("hull");

let dist = gjk_distance(&pts, &other_pts);
// 0.0 = overlapping
```

### Text — SDF layout

```rust
use toolkit_text::{AtlasBuilder, layout_text, LayoutOptions};

let mut builder = AtlasBuilder::new(1024);
// builder.add_glyph(/* coverage mask from your rasterizer */);
let atlas = builder.build();

let layout = layout_text(&atlas, "Hello\nWorld",
    &LayoutOptions { scale: 32.0, max_width: Some(200.0), line_gap: 4.0 });
// layout.glyphs: Vec<PositionedGlyph> with position + UV rect
```

### L-systems

```rust
use toolkit_lsystem::{LSystem, interpret, TurtleConfig};

let mut ls = LSystem::new("F");
ls.rule('F', "F[+F]F[-F]F");
let string = ls.expand(4);
let segments = interpret(&string, &TurtleConfig::default());
// segments: Vec<Segment> for rendering as lines or instancing geometry
```

### WFC — wave function collapse

```rust
use toolkit_wfc::{WfcModel, solve};
use toolkit_rng::Rng;

let mut model = WfcModel::new(2);
// model.allow(0, Dir::Right, 1); ... add adjacency rules
let rng = Rng::seed_from_u64(42);
if let Some(grid) = solve(&model, 16, 16, rng) {
    // grid: Vec<usize> in row-major order
}
```

## Adding AI/LLM Access

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use toolkit_ai_bridge::*;
use toolkit_ai_bridge::adapters::*;

// Wrap state in Arc<RwLock<T>> for shared access
let document = Arc::new(RwLock::new(Document::new("My Doc", 1920, 1080)));
let camera = Arc::new(RwLock::new(camera));
let orbit = Arc::new(RwLock::new(orbit));

// Build the bridge registry
let mut registry = BridgeRegistry::new();
registry.register(DocumentBridge::new(document.clone()));
registry.register(CameraBridge::new(camera.clone(), orbit.clone()));
// ... add more adapters as needed

// Start MCP server (runs on stdio, blocks)
let mut server = McpServer::new(registry);
server.run_stdio().unwrap();
```

The MCP server runs on stdin/stdout, allowing any MCP-compatible LLM client to read and modify your app's state through semantic tools and resources.

## Architecture Tips

1. **Keep state centralized** in your `Document` — don't scatter layer data across multiple places
2. **Use the command dispatcher** for all state mutations — this makes undo/redo and MCP integration work automatically
3. **Wrap shared state in `Arc<RwLock<T>>`** if you need MCP access or multi-threaded rendering
4. **Don't depend on modules you don't use** — the toolkit is designed for a la carte usage
5. **Build the BVH once, query many times** — rebuild only when the mesh changes
6. **Prefer `toolkit_meshops` for simple mesh cleanup** — it's cheaper than going through half-edge topology; save `toolkit_topology` for subdivision and adjacency edits
7. **`toolkit_attributes` replaces hard-coded fields** on geometry — use it for sculpt masks, paint weights, multi-UV sets, and per-element sim data
8. **`toolkit_field` unifies** noise, SDF, volume, and image sampling — pass any of these through the same `Field` trait to build composable pipelines
9. **Determinism matters** — `toolkit_rng` and `toolkit_noise` are seed-reproducible; same seed, same output, across platforms
