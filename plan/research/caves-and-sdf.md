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

## What S-161 built (2026-11-15)

The graph answers the queries, so the cave layer stores no RLE
columns. The code is in `rust/worldgen/src/caves/`.

- **Shapes.** A chamber is an ellipsoid with a flat floor plane. Its
  distance is the estimate `k0 * (k0 - 1) / k1` of Inigo Quilez. The
  code clamps the estimate between two simple bounds, so the bound
  check stays safe. A tunnel is a chain of capsules with a linear
  radius. Only the formulas come from the articles; the code is our own.
- **Smooth union.** The polynomial smooth minimum of Inigo Quilez. A
  chain of joins is never more than `k` below the smallest value.
- **A rule that keeps chunks equal.** A shape counts at a point only if
  its distance there is less than its reach (6 m + 2k + the bumps). The
  rule depends only on the point. Thus the grid, a chunk fill and a
  loop over all shapes give the same bits (a test checks this).
- **Bound check.** Each shape gives the lowest and highest distance in
  a ball around the block, from its Lipschitz constant. The 32 m blocks
  of seeds 1, 7 and 42 (4 m test zones): about 92% have no cave, and
  about 35 to 40% of the rest have a wall on a 4 m grid.
- **Noise.** Our own 3D gradient noise with the 12 cube-edge gradients
  of Ken Perlin's improved noise (2002), 4 octaves, 40 m.
- **Sky visibility.** Dijkstra on the graph gives the distance to the
  nearest mouth. The light falls exponentially along the tunnels. This
  is cheap and smooth, but it is not a ray cast.

| Source | Use | Licence |
|---|---|---|
| Inigo Quilez, "distance functions" and "smooth minimum", https://iquilezles.org/articles/ | Ellipsoid estimate, smooth union (formulas only). | Articles; no code copied. |
| Ken Perlin, "Improving Noise" (SIGGRAPH 2002) | Gradient set and fade curve. | Paper; no code copied. |
