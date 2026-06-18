# Implementation Tracker (temporary)

Working checklist for the "future modules" build. Each module: follow
`docs/creating-modules.md`, every file has `#[cfg(test)] mod tests`, lib.rs has a
doctest, derive `Serialize`/`Deserialize` on public data, reuse `toolkit_core`
IDs / `ToolkitResult`. **Commit after each module, then tick it here.**

Legend: `[ ]` todo · `[~]` in progress · `[x]` done (committed)

---

## Tier A — Substrates (build first; everything leans on these)

- [x] **toolkit_attributes** — named typed attribute channels bound to a domain
  (point/vertex/edge/face/primitive/detail). f32/i32/vec2/vec3/vec4/color/string.
  The Houdini-style backbone: sculpt masks, paint weights, per-element sim data,
  soft-selection weights, multiple UV sets.
- [x] **toolkit_volume** — dense 3D scalar/vector grid `Volume<T>` with trilinear
  sampling, gradient, resample, world transform. 3D fluids, voxel sculpt, 3D
  erosion, SDF baking, density fields.
- [x] **toolkit_field** — tiny `Field` trait (sample scalar/vector at a point) +
  combinators (add/mul/min/max/warp/clamp/remap). Unifies noise/sdf/volume/image
  as composable fields.
- [x] **toolkit_solver** — sparse matrix + iterative solvers (CG, Gauss-Seidel,
  Jacobi). Generalizes the CGLS embedded in toolkit_uv. Pressure projection,
  Laplacian deformation/smoothing, cloth, UV.

## Tier B — Interaction & editing operators

- [x] **toolkit_brush** — stroke + falloff engine: profile curves, radius/
  strength/spacing/pressure, stroke sampling along a path, apply-kernel-in-radius.
  Shared by sculpt / mesh paint / terrain / weight paint.
- [x] **toolkit_meshedit** — poly-modeling ops on the half-edge mesh: extrude,
  inset, bevel, bridge, loop-cut, dissolve, fill-hole. Normal modeling.
- [x] **toolkit_select** — unified typed selection sets (point/edge/face) with
  boolean ops, grow/shrink, by-attribute, and **soft selection** (falloff weights).
- [x] **toolkit_polyline** — 2D/3D polyline ops: resample by arc length, smooth
  (Chaikin/Laplacian), simplify (Douglas-Peucker), offset, length.

## Tier C — Geometry generation / processing

- [x] **toolkit_triangulate** — robust 2D polygon triangulation (ear clipping with
  holes + constrained Delaunay). 2D procedural shapes, CAD profile faces.
- [x] **toolkit_surfacing** — surface generation from profiles: extrude, loft,
  revolve, sweep-along-curve. CAD + procedural.
- [x] **toolkit_voxelize** — mesh -> volume (surface + solid / winding-number
  inside test). Bridges meshes into toolkit_volume for remesh, sims, booleans.
- [ ] **toolkit_remesh** — isotropic remesh + QEM decimation (the deferred QEM).
  Dynamic sculpt topology + cleanup.
- [ ] **toolkit_marching** — higher-quality isosurfacing (marching cubes + dual
  contouring with sharp features) over Field/Volume.
- [ ] **toolkit_deform** — procedural deformers: FFD lattice, bend/twist/taper,
  displace-by-field. Modeling + anim + procedural.

## Tier D — Simulation substrate

- [ ] **toolkit_particles** — particle state (SoA position/velocity/age/mass +
  attribute channels), force accumulation, integrators (Euler/Verlet), emitters.
  Substrate for SPH/FLIP, VFX, scatter sims, point-based erosion.

## Extra smaller modules (replacing Tier E "mesh boolean", deferred)

- [ ] **toolkit_scatter** — point distribution: Poisson-disk (2D/3D), jittered
  grid, area-weighted sampling of points on a mesh surface. Instancing, sockets,
  sim seeding.
- [ ] **toolkit_path** — frames along a curve (parallel-transport / Frenet),
  arc-length reparam, place transforms / instance geometry along a path
  ("mesh on path").
- [ ] **toolkit_sockets** — named attachment points (sockets): parent-relative
  transforms resolved to world space; attach to skeleton joints or mesh elements.

---

### Deferred (not in this pass)
- toolkit_meshbool (mesh CSG) — large; can be approximated via voxelize + marching.

### Notes / decisions
- Reuse existing: toolkit_graph (node DAG), toolkit_simulation (2D fluid/erosion),
  toolkit_state (history/layers), toolkit_core (commands/IDs/blend).
- Field trait kept dependency-free; concrete sources (noise/volume/sdf) provide
  adapters so toolkit_field stays a thin substrate.
