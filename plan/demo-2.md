# Demo 2: water and caves

Status: draft, 2026-11-15, from Brain row S-142. Mikey decided the
goal (R3, R4, R24, R25). The method and the items marked **assumed**
are proposals. The research is in the Workshop Wiki (R26); this file
keeps only the spec.

## The goal

The zone of demo 1 gets water and caves. Rivers, ponds and lakes fill
the valleys. Waterfalls drop over cliffs, and some are very tall or
very wide. The water has different colours and thicknesses, and the
bear swims in it. Large caves go deep below the mountains, and the
bear walks, climbs and swims in them. The game still runs at 60 fps at
1080p on Intel UHD Graphics (R5).

Demo 2 is done when Mikey swims across a lake, climbs out, finds a
cave mouth and goes down into a big chamber, and wants to go on.

## What Mikey sees first

1. **After S-160:** still water. Each zone has lakes and ponds in its
   basins, in different colours. The bear floats in deep water (a
   simple float, not yet the full swim).
2. **After S-164:** the first walkable cave. A hillside mouth leads
   down a tunnel to a chamber. The cave is lit like the outside, and
   it is not dark yet.
3. **After S-162 and S-163:** rivers that flow, the real swim and wet
   fur.

Play row S-167 is Mikey's first look at all of these.

## Build order

The cave rows and the water rows use different systems. Thus two
workers can do them at the same time. "Local" means a Claude Code
session on Mikey's laptop. It can start the game on the Intel GPU and
measure the GPU time itself.

| Row | Job | Needs |
|---|---|---|
| S-160 | Still water: lakes, ponds, the water shader, a simple float, the Water view test | – |
| S-161 | The cave layer in `bf-worldgen` (Rust only, preview images) | – |
| S-162 | Rivers with water, flow and rapids | S-160 |
| S-163 | Swimming and wet fur | S-160 |
| S-164 | The first walkable cave: meshes, collision, mouths, the Cave tour test | S-161 |
| S-165 | Waterfalls | S-162 |
| S-166 | Cave light | S-164 |
| S-167 | Play: Mikey's first look | S-160, S-163, S-164 |
| S-168 | Pools in caves | S-160, S-166 |
| S-169 | The far walls of big chambers (cave level of detail) | S-164 |
| S-170 | The ragdoll, lost limbs and swim gaits in water | S-163 |
| S-171 | Autoplay for water and caves | S-163, S-164 |
| S-172 | Obstacles with water and caves | S-132, S-162, S-164 |
| S-173 | Play: the demo 2 playtest | all build rows |
| S-174 | Brain: tuning rows from S-167 (numbers S-175 to S-179) | S-167 |

## The systems

The wiki pages explain each system. The spec here gives the rules
that the build rows must keep.

### Water bodies (S-160, S-162)

- Use the drainage of demo 1 (`drainage.rs`). Change `partial_fill`
  so that it also gives a record for each basin: its cells, floor,
  spill height and spill cell.
- **Lakes:** each deep basin gets a seeded fill part from 0.2 to 1.0.
  The level is `floor + part × (spill − floor)`. A full lake (part 1)
  spills at its spill cell. A closed lake (part below 1) gives no
  water out.
- **Ponds:** about 30% of the shallow basins (assumed) keep their
  hollow. The level is the spill height minus 0.1 to 0.5 m.
- **Rivers:** do a second flow accumulation in which each closed lake
  is a sink. A riverbed cell has water if its wet area is more than
  `wet_min_area_m2` (start value 150 000 m², assumed). Smaller beds
  stay dry.
- **River surface:** bed height plus a seeded part (0.4 to 0.8) of the
  bed depth. The surface never rises downstream. A drop of more than
  0.5 m from one cell to the next is a rapid. A drop of more than
  2.5 m, or a drop steeper than 45°, is a waterfall.
- **Ground:** a damp band along each shore in the colour map, and a
  water value in `Ground`.

### Water look (S-160, S-162)

- One `BfWater` node. Rust makes one mesh for each lake and each river
  system, with the water depth, the flow vector and a foam value in
  the vertex data.
- Do not read the screen texture or the depth texture. Do not use
  reflections or refraction. The colour and the opacity come from the
  depth in the vertex and the clarity of the body.
- Rivers use two ripple layers, each half a phase apart (the flow map
  method of Portal 2), with the flow from the vertices.
- At most 20 draw calls for water.

### Waterfalls (S-165)

- A waterfall comes from a river step or from the spill cell of a full
  lake above a steep drop.
- Each zone tries to make one tall fall (40 m or more) and one wide
  fall (30 m or more). It carves a flat lip at the best candidate. If
  no candidate is good, the zone has none of that kind, and the zone
  report says so.
- Draw the curtain as a mesh along the path of falling water, with a
  scrolling noise texture and alpha scissor. Add a foam ring on the
  pool, 32 to 64 splash particles, and 3 to 8 mist cards.
- Thick water falls as a few ropes, slowly, with no mist.

### Swimming (S-163)

All values are `#[export]` fields with these first guesses (assumed).

| State | Depth at the feet | Rule |
|---|---|---|
| Shallow | below 0.15 m | Walk. Speed × (1 − 0.5 × viscosity). |
| Wading | 0.15 to 0.25 m | Walk at 60%. Splashes. |
| Swimming | from 0.25 m (out below 0.22 m) | Float, paddle at 0.8 m/s (jog 1.3 m/s). |

- **Float line:** 0.18 m below the surface when dry, 0.08 m lower when
  soaked, divided by the density of the body. A soft spring holds it.
- **Viscosity** (0 to 1): the top speed falls to 25% at 1. The
  acceleration falls and the damping rises. At more than 0.5, the
  normal paddle uses stamina.
- **No stamina:** the bear floats and paddles at 0.3 m/s. It does not
  sink or drown (assumed, see the questions).
- **Current:** add the flow velocity of the river at the bear.
- **Banks:** if the bank top is less than 0.30 m above the water and
  walkable, pull up in 0.35 s (more in thick water). If the bank is
  steep, start the climb from the water (R11).
- **Wet fur:** each part below the surface gets wet 1.0. Each part
  dries in about 60 s in the sun, three times slower in a cave.

### The cave layer (S-161)

- Two to five cave systems in each zone. Chambers 40 to 250 m across
  and 20 to 120 m high. Tunnels 3 to 25 m wide. Each system goes down
  250 to 400 m below its highest mouth. All inside the safe part of
  the zone, away from the smoke edge.
- A seeded graph: chamber nodes (flattened balls with noise) and
  tunnel edges (capsules with a changing radius, bent by noise),
  joined with smooth union.
- The SDF is not stored. Compute it from the graph when a chunk needs
  it. Use the noise of `bf-worldgen` (the same bits on all computers).
- A spatial grid lists the shapes near each block. A chunk with no
  shape near it is solid, and it is skipped (the bound check).
- Queries without a mesh: `in_cave(p)`, `floor_below(p)`,
  `sky_visibility(p)`, `chamber_at(p)`.
- The roof above a cave is at least 8 m thick, except at mouths
  (assumed).
- The preview tool writes horizontal and vertical slices of the
  caves.

### Walkable caves (S-164)

- `fast-surface-nets` on 32 m chunks with 1 m voxels (34³ samples with
  the padding), on the `rayon` pool.
- Meshes within 96 m of the camera. Collision within 20 m of the bear.
  A chunk gets its collision before its mesh.
- The main thread makes at most 2 meshes and at most 1 collision shape
  in each frame. Each upload frame stays under 1 ms of CPU time.
- At most 80 draw calls for caves. Cave meshes cast no shadows.
- **Mouths:** a control image with the hole bit (bit 3, `1 << 2`) in
  all cells of each mouth and a band of 1 to 2 cells around it. In
  the band, the cave mesh follows the ground surface. Move the mesh
  edge a few centimetres down under the ground to hide cracks.
- Change the code that uses the heightmap as the ground: the put-back
  in `BfBear`, the "through the ground" event, the follow camera (ray
  cast in caves), the start point, the scatter and the obstacles
  (none in hole cells).

### Cave light (S-166)

- Bake the sky visibility (0 to 1) into the vertex colour of the cave
  meshes. The cave shader writes it to `AO` and sets
  `AO_LIGHT_AFFECT = 1.0`.
- The cave shader uses `fog_disabled` and its own dark fog.
- The fur shader and the water shader get the sky visibility as one
  uniform from the cave graph.
- Light sources: glowing minerals (emission in the vertex colour) until
  Mikey selects (assumed).
- **The deep-cave cut:** when the sky visibility at the camera is below
  0.02 and no mouth is in view, hide the Terrain3D node and turn off
  the sun's shadows. Measure it on and off, each in a new process.

### Pools in caves (S-168)

- Flat water at the low points of chamber floors, with a seeded level
  and look. Use the same `BfWater` meshes and shader, with the sky
  visibility of the chamber.

## The data for each water body and each cave

All values come from the zone seed and the number of the body. Number
the bodies in the order of their spill cells (or their first cell),
so the same seed gives the same list.

**Water body:** number, kind (lake, pond, river system, cave pool),
level (or a level for each river cell), cells, area, largest depth,
look kind, deep colour, clarity in metres (0.2 to 8 m), viscosity
(0 to 1), density (1.0 to 1.6), temperature (0 to 1; steam above
0.7), flow speed (rivers).

**Look kinds** (assumed; Mikey can change the list):

| Kind | Viscosity | Where it is more common |
|---|---|---|
| Clear spring | 0.0 | small ponds, caves |
| Silty brown | 0.1 | rivers |
| Milky alkaline | 0.15 | closed lakes |
| Rust orange | 0.2 | rivers below ash rock |
| Green algae | 0.3 | still ponds |
| Grey mud | 0.6 | shallow ponds |
| Black tar | 0.85 | rare, closed basins |

**Waterfall:** number, top point, bottom point, width, height, lip
direction, flow, the body above, feature fall (yes or no).

**Cave system:** number, nodes (position, radius, chamber or joint),
edges (two nodes, start and end radius, noise seed), mouths (position,
kind: hillside, sinkhole or crumbling floor), pools (chamber, level,
look), lowest and highest point.

## The GPU budget (Intel UHD Graphics, 1080p, Mobile renderer)

The frame now needs about 12 to 13.4 ms of GPU time with the scatter
(`research/gpu-speed.md`). The limit on a 60 Hz screen is 16.6 ms.
These are targets for the worst view of each system. A local build row
measures its system on the Intel GPU (`--gpu-index 0`) before it
pushes.

| System | Target, worst view | Worst view |
|---|---|---|
| Lakes, ponds and rivers | 1.0 ms | a lake on half of the screen |
| Waterfalls | 0.8 ms | 15 m from the largest fall |
| Swim splashes and ripples | 0.2 ms | the bear swims near the camera |
| Caves seen from outside | 0.3 ms | a mouth near the camera |
| Caves from inside | 1.5 ms | the middle of the largest chamber |
| Cave light | 0 ms | baked; a bear light is not measured |

- **Outside:** the worst view with water and a waterfall must stay
  below 15.5 ms. The Benchmark average must stay below 14 ms.
- **Inside a cave:** the frame must stay below 15.5 ms without the
  deep-cave cut. The cut is a saving, not a need.
- **CPU:** the zone generation gets at most 2 s more for water and
  caves. Uploads of cave meshes and collision stay under 1 ms in each
  frame.

**The cuts, in order, if a system is over its target:**

- Water: one ripple layer, then no shore fade, then opaque deep water,
  then unshaded water.
- Waterfalls: fewer mist cards, then no particles, then far falls as
  one card, then no mist.
- Caves: a mesh radius of 64 m, then 2 m voxels from 32 m, then the
  deep-cave cut always on deep in a cave, then no cave fog.

## Speed tests: the Tests page and the recorder

**New tests on the Tests page** (R22: Mikey only starts them):

- **Water view (S-160, more views in S-162 and S-165):** for seeds 1,
  7 and 42, the camera stops at fixed views: the largest lake, a river
  with rapids, the tallest fall and the widest fall. Each view lasts
  6 s, and the average starts after 2 s. The test writes the GPU time
  and the draw calls of each view. It runs each setting (water on,
  water off, waterfalls off) in a new process.
- **Cave tour (S-164, more in S-166 and S-169):** for seeds 1, 7 and
  42, the camera flies a fixed path: from outside to the largest
  mouth, down the tunnel, around the largest chamber and back. The
  test writes, for each part of the path, the GPU time, the draw
  calls, the meshed chunks, the mesh queue, the upload time and the
  hitches. It saves a screenshot at each mouth. It runs with the
  deep-cave cut on and off, each in a new process.
- The Benchmark and the Draw distance test also write the water and
  cave draw calls.

**New recorder data** (`samples.jsonl`, each second): `water` (body
number, kind, depth at the bear, state: shallow, wade or swim,
viscosity), `wet` (the average wet value of the parts), and `cave` (in
a cave, sky visibility, chunks with a mesh, chunks with collision,
mesh queue, upload ms).

**New events:** `swim_start`, `swim_end`, `bank_exit`, `bank_stuck`
(the bear pushes at a bank for 3 s and does not get out),
`fall_push` (a waterfall pushes the bear), `cave_enter`, `cave_leave`,
`cave_no_collision` (the bear is within 4 m of a cave chunk with no
collision), `cave_cut_on`, `cave_cut_off`.

**`report.md`** gets a "Water" and a "Caves" section with counts of
these events and the time in each state.

**Tests in `bf-worldgen`** (no Godot): the same seed gives the same
checksum of the water bodies and the cave graph. The water level is
above the ground in each cell of a body. A river surface never rises
downstream. The cave graph stays in the safe part of the zone. The
roof is thick enough except at mouths. Over seeds 1 to 50, count the
zones with a lake, a wet river, a tall fall, a wide fall and a mouth
within 600 m of the start point.

## What demo 2 does not have

- Diving under water. The bear floats only (assumed).
- Underground rivers, and rivers that fall into a sinkhole.
- Caves behind waterfalls.
- Caves that change while the game runs (digging, collapse), except
  the crumbling floor of R23.
- Sound of water (Driftloom comes later).

## Done when

- [ ] Each of seeds 1, 7 and 42 has a lake, a wet river, a waterfall
      and a cave mouth that the bear can reach from the start point.
- [ ] Mikey swims in thin and thick water, gets out at a bank, and
      the bear's fur is wet and then dries.
- [ ] Mikey walks from a mouth into a chamber of 100 m or more, and
      the bear does not fall through the floor.
- [ ] The Water view and the Cave tour stay inside the GPU budget on
      the Intel GPU of the TUF. If not, there is a list of causes.
- [ ] Mikey plays for ten minutes (S-173) and writes his notes.

## Questions for Mikey (taste only)

1. **The colours of the water.** Should most water look natural, with
   a few strange bodies (milky, orange, black, oily)? Or should most
   water look strange?
2. **Light in deep caves.** How does the bear see? (a) Glowing
   minerals and moss. (b) Only daylight from mouths and shafts, so it
   is very dark. (c) A small light that the bear carries.
3. **Thick water.** Is mud or tar only slow, or can it be a danger,
   so that a tired bear sinks slowly and must fight its way out?

## The research (Workshop Wiki)

The pages are in `nowmichaelclark/wiki`, folder `Broken Friends/`:
Water bodies, Water rendering, Waterfalls, Swimming, Caves, Cave
mouths and Cave light. The background pages are
`Background/Depression filling`, `Background/Flow accumulation` and
`Background/Surface nets`. The start page is `Broken Friends.md`:
https://github.com/nowmichaelclark/wiki/blob/main/Broken%20Friends.md
