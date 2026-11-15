# Ground scatter: notes for S-108

Date: 2026-11-14. These notes tell how the game places rocks, pebbles
and dead shrubs, and why.

## The method that we use

1. **A hash grid, not a random list.** Each class has a global grid of
   cells. One cell holds at most one prop. A hash of the cell number and
   the zone seed gives the size, the place and the chance. The same seed
   always gives the same props. No state is needed.
2. **Chunks on demand.** A chunk is a block of cells. The game makes the
   chunks near the camera and frees the far ones. A chunk does not need
   its neighbours, because a prop stays inside its own cell.
3. **One MultiMesh for each chunk and shape.** A MultiMesh draws many
   copies of one mesh in one draw call. A chunk is one node, so the
   visibility range of Godot can hide the whole chunk at its distance.
4. **Bodies only near the bear.** A pool of `StaticBody3D` moves to the
   nearest rocks (a convex hull of the mesh). Thousands of static
   bodies for the whole zone would cost memory and start time.

## Numbers (defaults)

| Class | Cell | Chunk | Range | Chance | Size |
|---|---|---|---|---|---|
| Rock | 5 m | 160 m | 320 m | 0.35 | 0.18 to 3.2 m wide |
| Pebble | 0.9 m | 32 m | 50 m | 0.90 | 2.5 to 10 cm wide |
| Shrub | 4 m | 64 m | 110 m | 0.22 | 0.3 to 1.1 m high |

The chance is for average ground. The ground changes it (rock,
sand, lake bed, clumps of noise). A unit test checks that the whole 768 m test zone has 0.02 to 0.6
pebbles for each square metre. A cloud run at the start point (seed 1,
software GL, debug library) showed 23 rock chunks with 637 rocks, and a
chunk took 0.45 ms to make, with the old pebble chance of 0.30. [unverified:
the draw call count and the frame time on Mikey's laptop.]

## Shapes

- **Rock.** An icosphere with one subdivision (80 triangles) and random
  bumps. Variant 0 is round. Variant 1 is a flat slab with a flat top.
- **Pebble.** An icosahedron (20 triangles) with bumps.
- **Dead shrub.** About 100 to 200 triangles. The skeleton grows by
  space colonization (Runions and others, 2007: "Modeling trees with a
  space colonization algorithm"). A cloud of attractor points fills the
  crown. Each branch tip grows one step toward the attractors nearest
  to it. The radius follows the pipe model (a branch is as thick as the
  branches above it together). Each branch is a prism with 3 sides.

## Sources

- **TanTan, `shrubbery`** (GitHub `TanTanDev/shrubbery`). It uses space
  colonization for plants. We did not use it. Its licence field shows
  "NOASSERTION" in the GitHub search, so the licence is not clear. The
  crate with the same name on crates.io is a behaviour-tree library, not
  this one. The algorithm is public, and our version is small
  (`props.rs`, about 100 lines).
- **Godot MultiMesh and visibility ranges.** A `GeometryInstance3D` has
  `visibility_range_end`. For a MultiMesh node, the range applies to
  the whole node. Godot measures the distance to the middle of the node
  [unverified]. Thus each chunk node sits in the middle of its chunk,
  and its range gets half the diagonal of the chunk as a margin.
- **`MultiMesh.buffer`.** The buffer for a 3D transform with colours
  has 16 numbers for each instance: 3 rows of 4 (the origin is the last
  number of each row) and 4 for the colour. The node checks the first
  instance after the first buffer and prints an error if it is wrong.

## What to measure on Mikey's laptop

1. The overlay line `scatter:` shows the chunks, the props, the bodies
   and the build time of one chunk. The draw calls are in the line
   above it.
2. Turn the scatter off in the dev menu (last item of the main page).
   The difference in frame time is the cost of the scatter.
3. In `Scatter` (Inspector), `rock_range_m`, `pebble_range_m` and
   `shrub_range_m` set how far each class draws. `rock_density`,
   `pebble_density` and `shrub_density` set how many.
