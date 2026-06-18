Tier 1 — highest value, genuinely small, broad reuse
SOme of this are done and some are not !!!!!!!!!!!!
┌─────────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────┬────────────────────────────┬──────┐
│     Module      │                                                What it gives                                                │         Pairs with         │ Size │
├─────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────┼──────┤
│ toolkit_curves  │ Bézier / B-spline / NURBS curves + surfaces (evaluate, derivative, knot insertion, tessellate→Mesh). This   │ geometry, topology         │ S–M  │
│                 │ is the CAD foundation you were about to start.                                                              │                            │      │
├─────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────┼──────┤
│ toolkit_noise   │ Perlin / Simplex / Worley / fBm. Terrain, textures, procedural everything. Pure functions, zero deps.       │ simulation, sdf, uv        │ S    │
├─────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────┼──────┤
│ toolkit_rng     │ Deterministic seeded PRNG (PCG/xoshiro) + distributions + sampling (disk, sphere, poisson). Reproducible    │ noise, simulation, wfc     │ S    │
│                 │ procedural work.                                                                                            │                            │      │
├─────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────┼──────┤
│ toolkit_sdf     │ Signed-distance primitives + boolean ops + marching cubes→Mesh. The core of the SDF editor you mentioned.   │ geometry, noise            │ M    │
├─────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────┼──────┤
│ toolkit_meshops │ Non-topological mesh utils: weld/merge-by-distance, recompute normals/tangents, flip winding, QEM           │ geometry, topology, assets │ S–M  │
│                 │ decimation, Laplacian smooth, stats.                                                                        │                            │      │
├─────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────┼──────┤
│ toolkit_spatial │ Spatial hash grid + kd-tree + octree for nearest-neighbor / range queries.                                  │ geometry, simulation,      │ S    │
│                 │                                                                                                             │ gizmo picking              │      │
├─────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────┼──────┤
│ toolkit_anim    │ Keyframe tracks + interpolation + clip/timeline sampling. Drives scene transforms.                          │ scene, easing              │ S–M  │
├─────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────┼──────┤
│ toolkit_easing  │ Easing curves + tweening (cubic, elastic, spring). Tiny, used everywhere (UI, camera, anim).                │ canvas, anim, render nav   │ S    │
└─────────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────┴────────────────────────────┴──────┘

Tier 2 — useful glue, small

┌───────────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┬──────┐
│      Module       │                                                              What it gives                                                              │ Size │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_project   │ Save/load a whole project bundle (scene + meshes + materials + UVs) as one serialized file. The missing "open/save my work."            │ S–M  │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_units     │ Unit system (mm/cm/m/inch), conversions, measurement — important for CAD precision.                                                     │ S    │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_intersect │ Broader intersection/closest-point tests (segment, sphere, plane, capsule) + frustum culling for AABB/sphere. Extends geometry's        │ S    │
│                   │ ray-only set.                                                                                                                           │      │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_color     │ Extend core's LinearRgba with HSV/OKLab, gradients, palettes, color ramps.                                                              │ S    │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_image     │ CPU image buffer: blit, resize, sample, PNG read/write. Needed for texture work + baking.                                               │ S–M  │
└───────────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┴──────┘

Tier 3 — valuable but larger / more specialized

┌───────────────────────────────┬───────────────────────────────────────────────────────────────────────────────────┬──────┐
│            Module             │                                   What it gives                                   │ Size │
├───────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_skeleton              │ Joints, skin weights, pose/skinning. Unlocks character animation.                 │ M    │
├───────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_texture_bake          │ Rasterize UV charts → texture; AO/normal baking. Natural follow-on to toolkit_uv. │ M    │
├───────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_convex                │ Convex hull (QuickHull) + GJK distance — collision, bounds, physics.              │ M    │
├───────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_text                  │ SDF font atlas + text layout (labels, measurements, UI).                          │ M    │
├───────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────┼──────┤
│ toolkit_wfc / toolkit_lsystem │ Procedural content generators.                                                    │ M    │
└───────────────────────────────┴───────────────────────────────────────────────────────────────────────────────────┴──────┘