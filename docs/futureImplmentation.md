# Future Implementation — not yet built

Modules that have been scoped but are **not yet implemented**. Updated from
the old `implementation.md` tracker (all ticked items now exist on disk).

## Tier A — Substrates

*(All Tier A modules are implemented — see `module-reference.md`)*

## Tier B — Interaction & editing operators

*(All Tier B modules are implemented)*

## Tier C — Geometry generation / processing

- [~] **toolkit_marching** — higher-quality isosurfacing (marching cubes + dual
  contouring with sharp features) over Field/Volume.
  *(Crate directory exists but is empty — not yet implemented.)*
- [ ] **toolkit_deform** — procedural deformers: FFD lattice, bend/twist/taper,
  displace-by-field. Modeling + anim + procedural.

## Tier D — Simulation substrate

- [ ] **toolkit_particles** — particle state (SoA position/velocity/age/mass +
  attribute channels), force accumulation, integrators (Euler/Verlet), emitters.
  Substrate for SPH/FLIP, VFX, scatter sims, point-based erosion.

## Extra smaller modules (not yet started)

- [ ] **toolkit_scatter** — point distribution: Poisson-disk (2D/3D), jittered
  grid, area-weighted sampling of points on a mesh surface. Instancing, sockets,
  sim seeding.
- [ ] **toolkit_path** — frames along a curve (parallel-transport / Frenet),
  arc-length reparam, place transforms / instance geometry along a path
  ("mesh on path").
- [ ] **toolkit_sockets** — named attachment points (sockets): parent-relative
  transforms resolved to world space; attach to skeleton joints or mesh elements.

## Deferred (not in this pass)

- **toolkit_meshbool** (mesh CSG) — large; can be approximated via voxelize + marching.

## Original design notes (kept for reference)

These modules were originally listed as future possibilities.
**All of them are now implemented on disk** — see `module-reference.md`.

### Tier 1 — highest value, genuinely small, broad reuse

| Module | What it gives | Pairs with |
|--------|---------------|------------|
| `toolkit_curves` | Bézier / B-spline / NURBS curves + surfaces | geometry, topology |
| `toolkit_noise` | Perlin / Simplex / Worley / fBm | simulation, sdf, uv |
| `toolkit_rng` | Deterministic seeded PRNG + distributions | noise, simulation, wfc |
| `toolkit_sdf` | SDF primitives + booleans + surface nets → Mesh | geometry, noise |
| `toolkit_meshops` | Non-topological mesh utils: weld, normals, flip, smooth | geometry, topology, assets |
| `toolkit_spatial` | Spatial hash grid + kd-tree + octree | geometry, simulation, gizmo |
| `toolkit_anim` | Keyframe tracks + interpolation + scene transform drive | scene, easing |
| `toolkit_easing` | Easing curves + tweening | canvas, anim, render nav |

### Tier 2 — useful glue, small

| Module | What it gives |
|--------|---------------|
| `toolkit_project` | Save/load project bundle (scene + meshes + materials) |
| `toolkit_units` | Unit system (mm/cm/m/inch) for CAD precision |
| `toolkit_intersect` | Broader intersection tests + frustum culling |
| `toolkit_color` | HSV/OKLab, gradients, palettes, color ramps |
| `toolkit_image` | CPU image buffer: blit, resize, sample, PNG I/O |

### Tier 3 — valuable but larger / more specialized

| Module | What it gives |
|--------|---------------|
| `toolkit_skeleton` | Joints, skin weights, pose/skinning |
| `toolkit_texture_bake` | UV raster → AO/normal baking |
| `toolkit_convex` | Convex hull + GJK distance |
| `toolkit_text` | SDF font atlas + text layout |
| `toolkit_lsystem` / `toolkit_wfc` | Procedural content generators |

## Future Baking Implementations
- **GPU Compute Shaders**: Currently, texture baking (AO, curvature, normals) is heavily parallelized across CPU cores using Rayon. While this scales perfectly with high core count CPUs, we should ultimately implement WGPU Compute Shaders to run the physics BVH raycasting on the user's graphics card, which will drop bake times from seconds to milliseconds.
