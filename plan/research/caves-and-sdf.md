# Research: caves and SDF structures (demo 2)

From the Research run of 2026-11-14. Check each licence again before
you copy code.

## Method to use

- **Storage (RLE columns):** each (x, z) column stores the surface
  height from the heightmap and a short list of air spans (y_min,
  y_max, material). Most columns have no spans.
- **Density:** `sdf = max(terrain_sdf, -cave_sdf)`. The cave SDF comes
  from a cave graph: seeded points joined by capsules, with
  low-frequency noise, and smooth-min for big chambers. Arches, land
  bridges and statues are SDF shapes (torus, capsules, smooth union)
  with domain warp, placed by the seed.
- **Speed (TanTan's list):**
  1. A bound check for each chunk. If the chunk is fully solid or
     fully empty, skip it.
  2. Noise up-sampling: sample on a grid 4 times coarser and
     interpolate.
  3. A noise cache: keep the 2D noise of each column for all y.
  4. Mesh only chunks that contain a change of sign.
- **Mesh:** `fast-surface-nets` on 16^3 chunks (18^3 with padding), on
  a `rayon` pool. On the main thread, build the `ArrayMesh` from
  packed arrays. Upload only a few chunks each frame.
- **Collision:** only for chunks within about 10 to 20 m of the bear.
  `ConcavePolygonShape3D::set_faces` needs a flat list of triangles.
  Build the arrays on worker threads.

## Sources

| Source | Use | Licence |
|---|---|---|
| fast-surface-nets-rs, https://github.com/bonsairobo/fast-surface-nets-rs | The cave mesher. | MIT OR Apache-2.0 |
| bonsairobo/feldspar, https://github.com/bonsairobo/feldspar | `grid-tree`, `grid-ray` and chunk patterns. | MIT |
| transvoxel_rs, https://github.com/Gnurfos/transvoxel_rs | Only if caves need LOD seams. | MIT OR Apache-2.0 |
| Matt Keeter, fidget and the MPR paper, https://www.mattkeeter.com/research/mpr/ | Interval arithmetic to skip empty or full blocks. | MPL-2.0 |
| Zylann/godot_voxel, https://github.com/Zylann/godot_voxel | Godot chunk streaming, collision and threads (C++, read only). | MIT |
| Veloren devblogs, https://veloren.net/blog/devblog-16/ | Column storage ("chonks"). | GPL-3.0. Study only. |
| Godot docs, Using the ArrayMesh | The glue from Rust to Godot. | Docs. |
