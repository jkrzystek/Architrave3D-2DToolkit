# The 3D Pipeline

A worked example chaining the new crates into one flow: **import → edit topology
→ unwrap UVs → place in a scene → manipulate with a gizmo**. Every step is
independent, so you can drop into the pipeline wherever you need.

```
assets ──► geometry ──► topology ──► uv ──► (texture atlas)
              │             │
              └──► scene ◄──┘
                     │
                  gizmo  (edit transforms)
```

## 1. Import a model

```rust
use toolkit_assets::{import_gltf_path, import_obj_path};

// Any importer yields a format-neutral ImportedScene.
let imported = import_gltf_path("assets/robot.glb")?;   // or import_obj_path(...)
println!("{} meshes, {} triangles", imported.meshes.len(), imported.triangle_count());

// Build a scene graph of nodes referencing the meshes by id.
let mut scene = imported.build_scene();
scene.update_world_transforms();
```

## 2. Edit topology (subdivide / clean up)

```rust
use toolkit_topology::HalfEdgeMesh;

let mesh = &imported.meshes[0];
let he = HalfEdgeMesh::from_mesh(mesh);

// Smooth it with one Catmull-Clark step, then get a renderable mesh back.
let subdivided = he.catmull_clark();
let render_mesh = subdivided.to_mesh("robot_smooth");

// Adjacency queries are now available:
println!("genus check (Euler): {}", subdivided.euler_characteristic());
```

## 3. Unwrap UVs

```rust
use toolkit_topology::{HalfEdgeMesh, MeshSelection, SelectMode};
use toolkit_uv::{unwrap_charts, pack_charts};

// Pull positions + triangles out of the mesh.
let positions: Vec<_> = mesh.vertices.iter().map(|v| v.position_vec3()).collect();
let triangles: Vec<[usize; 3]> = mesh.indices
    .chunks_exact(3)
    .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
    .collect();

// Mark seams (here: none -> a single chart). In an editor these come from a
// MeshSelection in Edge mode.
let seams: Vec<(usize, usize)> = vec![];

// Segment into charts and flatten each with LSCM, then pack into one atlas.
let mut charts = unwrap_charts(&positions, &triangles, &seams);
pack_charts(&mut charts, 0.01);

for (i, chart) in charts.iter().enumerate() {
    println!("chart {i}: {} tris, distortion {:.4}", chart.triangles.len(), chart.distortion());
}
// chart.uvs now hold atlas-space UVs per local vertex; map back via
// chart.local_to_global to write them onto your mesh.
```

For quick results without a solver, use the projection unwraps instead:
`project_planar`, `project_box`, `project_cylindrical`, `project_spherical`.

## 4. Display UVs in a 2D editor

```rust
use glam::Vec2;
use toolkit_canvas::{CanvasView, adaptive_step, SelectionDrag};

let mut view = CanvasView::new(Vec2::new(1024.0, 1024.0));
view.fit_bounds(Vec2::ZERO, Vec2::ONE, 0.1);  // frame the [0,1] UV square

// Convert a chart UV to a screen pixel for drawing.
let screen = view.canvas_to_screen(charts[0].uvs[0]);

// Cursor-anchored zoom and a readable grid:
view.zoom_at(Vec2::new(512.0, 512.0), 1.2);
let step = adaptive_step(&view, 64.0);
```

## 5. Select and transform with a gizmo

```rust
use glam::Vec3;
use toolkit_geometry::Ray;
use toolkit_gizmo::{Gizmo, GizmoMode, GizmoDelta};
use toolkit_scene::Transform;

// Place a gizmo at the selected node's world position.
let node = scene.roots()[0];
let start = scene.world_transform(node).unwrap();
let mut gizmo = Gizmo::new(start.translation, GizmoMode::Translate);

// On mouse-down: pick a handle with the camera pick ray.
let pick = Ray::new(Vec3::new(0.5, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
if let Some(handle) = gizmo.hit_test(&pick, /*view_dir*/ Vec3::NEG_Z) {
    gizmo.begin_drag(handle, &pick, Vec3::NEG_Z);

    // On mouse-move: apply the cumulative delta to the node's start transform.
    if let Some(GizmoDelta::Translate(offset)) = gizmo.update_drag(&pick, Vec3::NEG_Z) {
        if let Some(n) = scene.get_mut(node) {
            n.transform = Transform { translation: start.translation + offset, ..start };
        }
    }
    gizmo.end_drag();
}
scene.update_world_transforms();
```

## 6. Render with PBR

```rust
use glam::Vec3;
use toolkit_render::{PbrMaterial, PBR_SHADER_WGSL};

let gold = PbrMaterial::metal(Vec3::new(1.0, 0.76, 0.34), 0.2);
let uniforms = gold.uniforms();           // upload via bytemuck::bytes_of(&uniforms)
// PBR_SHADER_WGSL is a ready Cook-Torrance shader (bind groups: view / model / material+light).
```

## 7. Export

```rust
use toolkit_assets::export_obj_path;
export_obj_path(&imported.meshes, "out/robot.obj")?;
```

## Letting an LLM drive it

Wrap the scene in `Arc<RwLock<Scene>>` and register `SceneBridge` (see
[AI Bridge Guide](ai-bridge-guide.md)) so an MCP client can add nodes, move
them, and reparent — the same operations the gizmo performs, but driven by an
agent.

## Other Pipeline Paths

This was one path through the toolkit. Here are others:

```
SDF modeling:     primitives → CSG → polygonize → mesh
Procedural:       noise + rng + field → sdf / volume → remesh → mesh
CAD:              curves → surfacing → meshops → topology
Terrain:          noise → simulation(erosion) → brush → mesh
Scatter:          rng(poisson) → scene instancing
```

See the relevant crate docs in `module-reference.md`, or the quick-start
sections in `building-apps.md` for each pattern.
