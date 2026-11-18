# Agenda rows: the full text

`STATE.md` has one short line for each row. This file has the full text
of each row. A session reads its row here. At the end, it deletes the
row here and in `STATE.md`.

## S-124

**Build: playtest replay** (`diagnostics.md`, "Later: replay"). Record the input of each physics tick in the playtest. A headless command replays a playtest with its seed and compares the bear's path with the recorded positions.

Where: Claude Code. Model: Opus 5.5, high.

## S-126

**Build: tune the walk animation from the playtest.** Read Mikey's notes and `anim_us` in `samples.jsonl` of the playtest. Change the defaults on `BfBearModel` (stride, bounce, waddle, springs) and keep `AnimParams::default()` in `anim.rs` the same. Fix what looks wrong at the marks.

Where: Claude Code. Model: Sonnet 5.5, medium.

## S-130

**Build: tune the climbing hands and the gaits from the playtest** (R18). Read Mikey's notes and the `limb_off`, `gait` and `grab` events. Change the defaults of the holds (`hold_step_m`, `hand_reach`, `hang_*`) and of the gaits (hop, lie, crawl, the speed scales on `Bear`). Keep `AnimParams::default()` and `GaitTuning::default()` the same as the exported defaults.

Where: Claude Code. Model: Sonnet 5.5, medium.

## S-131

**Build: tune the smoke edge from the playtest** (R14, `demo-1.md`). Read Mikey's notes, `smoke_m` and `smoke_s` in `samples.jsonl`, and the events `smoke_enter`, `smoke_leave` and `death:smoke:*` with their shots. Change the defaults on `BfSmokeEdge` (limits, wall speed, opacity, haze, card count). Check the frame time near the wall (overdraw) and cut cards if it is high.

Where: Claude Code. Model: Sonnet 5.5, medium.

## S-137

**Play: the feel test on the Intel GPU of the TUF.** Download the newest zip with `tools/get-build.sh`. Start it with `--gpu-index 0`. The lead Brain already ran the three tests (`play-notes.md`, 2026-11-15). Play for 10 minutes. Send the zip, and one line each on: the new, cheaper look of the ground (S-139); the rocks, pebbles and dead shrubs (S-108), and if they pop in; the walk; the climb; the smoke wall.

Where: Mikey's TUF laptop. Model: Mikey.

## S-109

**Brain** with Mikey: play the full demo 1, decide what to fix. Demo 2 planning moved to S-142.

Where: Project chat. Model: Opus 5.5, medium.

## S-134

**Build: fix what the first nightly autoplay finds** (`diagnostics.md`, "Autoplay"). Read `autoplay-nightly.md` on the `game-latest` release and the "Autoplay" notes of the game builds. Fix the bugs that it shows (falls through the ground, errors, many stuck or climb loop events at one place). If a task of the tour is often "missed" on a seed, make the bot better (`playtest/autoplay.rs`). Also look at the small put-back (0.26 m) on seed 7 at the stepping stones near (-256, 201): the collision and the heightmap do not agree there.

Where: Claude Code. Model: Sonnet 5.5, medium.

## S-141

**Build: tune the scatter from the playtest** (`research/scatter.md`). Read Mikey's notes, the overlay line `scatter:` in the marks, and the frame time with the scatter on and off (dev menu). Change the defaults on `BfScatter` (density, size scale, ranges, collision) and `ScatterParams::default()` with them. If the draw calls or the chunk build time are too high on the Intel GPU, use the ideas in `inbox.md`. Also find the hitches of the Benchmark on seed 1 (one 400 ms frame on build ae685ec), the slow first 10 to 20 s of the first run (GPU 18 to 22 ms, not 12 ms), and the shader compiles in the first minute of play (`play-notes.md`, 2026-11-15).

Where: Claude Code. Model: Sonnet 5.5, medium.

## S-143

**Play: the integrated-graphics test on the Vivobook**, the same as S-137, when Mikey has the Vivobook again.

Where: Mikey's Vivobook. Model: Mikey.

## S-155

**Build: check the S-135 tear and tumble values from the next playtest.** Count the lost legs and the time on the ground in `report.md`. If legs still come off more than about 3 times in 30 minutes, lower `seam_damage_per_speed` again. Add a recorder line for the speed of each limb hit, so the next tune has data.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, medium.

## S-163

**Build: swimming and wet fur** (`demo-2.md`, "Swimming"). Replace the float of S-160: wade, swim, float line, viscosity, current, stamina, bank exit, climb from the water, wet parts that dry. Recorder events `swim_*`, `bank_*`. Parallel: S-161, S-162, S-164.

Where: Claude Code (local). Model: Sonnet 5.5, high.

## S-167

**Play: the first look at water and caves.** Download the newest zip. Start it with `--gpu-index 0`. Run the Water view, the Cave tour and the Benchmark, then play for 10 minutes. Send the zip and one line each on: the water colours, the swim, the first cave, the questions in `demo-2.md`. Also compare the new water (own light, opaque deep quads, no shadows on the water, S-180) with the old look: colour, ripples, sun glint, the bed through clear water (this was S-184). Do S-137 in the same sitting.

Where: Mikey's TUF laptop. Model: Mikey.

## S-168

**Build: pools in caves** (`demo-2.md`, "Pools in caves"). Water at the low points of chamber floors, with the cave light. Parallel: S-169 to S-172.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, medium.

## S-169

**Build: the far walls of big chambers.** A second level of cave chunks (2 m voxels from about 48 m), hidden seams, and occluders from the cave shells if the Cave tour shows overdraw. Parallel: the water rows.

Mikey's first list (2026-11-15, his words in short; he says there are more):

1. **The water does not follow the ground and looks square**, not natural. First idea [unverified]: the water edge follows the quads of the mesh (2 m for lakes, 1 m for rivers), and the shore fade reads the depth only at the vertices (weak spot of S-160). Look at lake and river shores on seeds 1, 7 and 42, on steep and on flat banks. S-181 (Speed) has the idea of a dithered shore edge; do the two rows together or one after the other, because both change `water.gdshader` and the water mesh.
2. **A thing like bright smog deep in the caves.** Mikey is not sure that it is the smog. First ideas [unverified]: the cave fog takes the smog colour too far from a mouth (S-166), or a thing from outside still draws in the deep cave (the smoke wall cards, the sky, dust, mist; the deep-cave cut hides only the terrain). Run the Cave tour, make pictures in the chambers of seeds 1, 7 and 42, and turn the parts off one at a time.
3. **The camera sometimes goes through the ground.** First idea [unverified]: the follow camera reads the heightmap, and a ray only near a cave (S-164), so rocks, crust lids, cave rims and steep ground behind the bear can cut the view (`inbox.md` has two lines on this). Add a recorder probe that counts the frames with the camera below the ground or in a mesh, with the place, so the autoplay finds the places.

Ask Mikey for the other bugs after these three, or read his marks in the next playtest.

Where: Claude Code (local). Model: Opus 5.5, high.

## S-170

**Build: the ragdoll, lost limbs and swim gaits in water** (R11, R12, R18). Lift and drag for each ragdoll part, floating lost limbs, a dog paddle from the limbs that the bear still has. Parallel: the cave rows.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, high.

## S-171

**Build: autoplay for water and caves** (`diagnostics.md`, "Autoplay"). Tour tasks: swim across a lake, get out at a bank, go into the nearest mouth, reach a chamber, stand in a fall. Caves in the nightly wander. Parallel: S-168 to S-170.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, medium.

## S-172

**Build: obstacles with water and caves** (R23). No obstacles in water or hole cells, a crumbling floor that drops the bear into a cave, stepping stones across rivers. Parallel: S-168 to S-171.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, high.

## S-173

**Play: the demo 2 playtest.** The same as S-167, with the waterfalls, the cave light, the pools and the far walls. Play for 10 minutes and send the zip and notes.

Where: Mikey's TUF laptop. Model: Mikey.

## S-174

**Brain: tune demo 2 from S-167.** Read the zip and Mikey's answers. Write his decisions with him, and add the tuning rows.

Where: Claude Code (local). Model: Opus 5.5, medium.

## S-150

**Build: tune the obstacles again from the next playtest.** Read "Obstacles" in `report.md`. Check the rolling stones (hits per tell; the lane is now 1 to 2.5 m wide), the ash crust (falls against escapes; the shake is now 1.8 s) and the launch vents (the events end with `:dy=` and `:off=`: if a launch still misses, read them, and look at the marks for a bear that hits the face). Then change the defaults on `Obstacles` and `SimParams::default()`. Also check the new steer after a launch (`launch_steer_s`): the bear must not feel stuck on the wall or lose the grab at the ledge lip.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, medium.

## S-181

**Build: cut the water to 1.0 ms or less.** With the rivers (build 2e2d5f8), the lake views cost 1.1 to 2.2 ms and the river views 1.1 to 1.8 ms (target 1.0, `play-notes.md`). Test these in new processes with `--bf-water-<name>=<value>`: the smog fog of the water with a cheap own formula (fog costs about 0.5 ms), a dithered shore edge instead of blended shore quads (no blending at all), and fewer far quads (a 8 m quad from 150 m). Keep the look.

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-182

**Build: a water sweep script.** `tools/water-sweep.sh` runs the Water view once for each setting in a list, each in a new process, alternates "water off" between them, and writes one table of GPU ms (average of the 3 seeds, off subtracted). The noise of one run is about 0.5 ms, so each setting runs 3 times.

Where: Claude Code (local). Model: Sonnet 5.5, low.

## S-183

**Build: why a lake view without water costs 14.5 ms.** The Water view without water is 2 ms above the Benchmark (12.5 ms). Find the cause (the bed under the lake, the open view, the far smog) with probes. Idea: draw the opaque water before the terrain, so that the bed under it is not shaded.

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-186

**Build: the cave rim looks like the ground.** Look at the mouths of seeds 1, 7 and 42 in the Mobile renderer: the hole edge, a step at the terrain edge, the colour and the grain. Fix the seams that show (`rim_ring_cells`, `rim_sink_m`, the colour).

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-187

**Build: a dev menu line for the caves** (on and off) and a "go to the nearest mouth" teleport, so Mikey finds a cave without a map.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, low.

## S-191

**Build: longer rivers, if Mikey wants them** (S-162 "Assumed" in `game/README.md`). The rivers are short, steep streams because closed lakes catch most of the rain. Try more full lakes (`full_lake_chance`), a lower `wet_min_area_m2` or wider valley floors, and compare the river length and the rapids of seeds 1, 7 and 42 (`print_rivers_for_wet_areas`).

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-192

**Build: tune the river look from the playtest.** Read Mikey's notes on the rivers of S-167: the foam (about half of the cells are rapids), the flow speed of the ripples, the step where a river meets a lake. Change the defaults on `BfWater` (`foam_strength`, `flow_cycle_s`, `flow_ripple_gain`) and in `WaterParams` (`rapid_drop_m`, `mouth_slope`).

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-195

**Build: the cave light on the water.** Read the global uniform `bf_cave_sky` (`caves.rs`) in `water.gdshader`: use it for the water `AO` and the water fog, so a pool in a cave is dark. S-166 left the water files alone because S-162 changed them. S-168 needs this. Parallel: S-168.

Where: Claude Code (local). Model: Sonnet 5.5, low.

## S-201

**Build: look at the falls and tune them.** Look at the tall and the wide fall of seeds 1, 7 and 42 in the Mobile renderer: the jagged edge where the sheet meets the ground, the colour of the curtain, the splash cards, the mist. Fix what shows (`ground_offset_m`, the mask fade in `waterfall_mesh.rs`, `streak_*`, `foam_*`).

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-202

**Build: true cliffs for the feature falls, if Mikey wants them** (S-165 "Assumed" in `game/README.md`). Carve a flat lip and a near-vertical drop with an overhang at the best candidate, and raise the river levels to match. Check that caves, obstacles and scatter still fit the carved ground.

Mikey's first list (2026-11-15, his words in short; he says there are more):

1. **The water does not follow the ground and looks square**, not natural. First idea [unverified]: the water edge follows the quads of the mesh (2 m for lakes, 1 m for rivers), and the shore fade reads the depth only at the vertices (weak spot of S-160). Look at lake and river shores on seeds 1, 7 and 42, on steep and on flat banks. S-181 (Speed) has the idea of a dithered shore edge; do the two rows together or one after the other, because both change `water.gdshader` and the water mesh.
2. **A thing like bright smog deep in the caves.** Mikey is not sure that it is the smog. First ideas [unverified]: the cave fog takes the smog colour too far from a mouth (S-166), or a thing from outside still draws in the deep cave (the smoke wall cards, the sky, dust, mist; the deep-cave cut hides only the terrain). Run the Cave tour, make pictures in the chambers of seeds 1, 7 and 42, and turn the parts off one at a time.
3. **The camera sometimes goes through the ground.** First idea [unverified]: the follow camera reads the heightmap, and a ray only near a cave (S-164), so rocks, crust lids, cave rims and steep ground behind the bear can cut the view (`inbox.md` has two lines on this). Add a recorder probe that counts the frames with the camera below the ground or in a mesh, with the place, so the autoplay finds the places.

Ask Mikey for the other bugs after these three, or read his marks in the next playtest.

Where: Claude Code (local). Model: Opus 5.5, high.

## S-203

**Build: the bear in a fall** (R11, R12, R18). Tune `fall_push_accel` from the recorder events `fall_in` and `fall_out`. Add the tumble, the wet parts and the splash on the bear when it stands in a curtain, and a spray when it enters the pool.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, medium.

## S-204

**Build: falls in the autoplay** (`diagnostics.md`, "Autoplay"). Add the tour task "stand in a fall" (go to the foot of the tallest fall within 580 m, then climb into the curtain). Count `fall_in` events in the report.

Where: Lead chat (cloud worker). Model: Sonnet 5.5, low.

## S-205

**Build: a Cave tour without caves, for the base cost.** With `--bf-caves=off` the Cave tour finds no cave and flies nowhere. Keep the cave data for the path, and hide only the cave meshes, rims and holes. Then the outside part gives the cost of the caves seen from outside (budget 0.3 ms).

Where: Claude Code (local). Model: Sonnet 5.5, medium.
## S-207

**Hygiene: make the docs agree with the plan.** `tech.md`, `demo-1.md`, `demo-2.md`, `diagnostics.md`, `local-setup.md` and `game/README.md` can have text from before demo 2 and from before the lead types (R27). Fix each doc; do not change a rule. The session briefs are already done (S-206).

Where: Claude Code (local or cloud). Model: Sonnet 5.5, medium.

## S-208

**Hygiene: the weak spots of the first demo 2 wave.** Read the weak spots in the commits of S-160 to S-166 and S-180. For each one, name the row that has it, or add a row for the correct lead, or write one line in `inbox.md`. Examples with no row: the water mesh build of 60 to 150 ms on the main thread (S-162), the stepped walls at river mouths (seed 7, river #49), the examples that only CI builds (S-161), the release library through a symlink (S-180).

Where: Claude Code (local or cloud). Model: Sonnet 5.5, medium.

## S-209

**Research: move `plan/research/` to the wiki** (R26). 15 notes. For each note, write one or more wiki articles in `Broken Friends/` (follow the wiki's `CLAUDE.md` and `Wiki/Writing guide.md`), keep in the repo only the result that a spec needs, change the links, and delete the note. Do 4 or 5 notes in a session, and keep this row until the folder is empty.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, medium.

## S-211

**Bird's-eye: the first look from above.** Sort `inbox.md` (delete what is built, group the rest by system, mark what fits `vision.md`). Give Mikey the list "Proposed, Mikey must decide" of `decisions.md` and the "Assumed" list of `game/README.md` as short taste questions; technical points that are in use for weeks become decisions if he agrees. Check the order of all agendas against demo 2 and the proc-gen direction. Write `STATE.md` again.

Where: Claude Code (local). Model: Opus 5.5, high.

## S-212

**Fix: the visual and collision bugs that Mikey sees now** (Mikey, 2026-11-15). Do this row after the tool rows S-213 to S-216. These three bugs are the test of the tools: a tool must show each one before a session fixes it. Mikey gives one line for each bug in his own words (what he saw, and about where), or taps a mark in a play. For each bug: find the seed and the place (the mark, the newest zips in `~/Games/broken-friends-linux/playtests/`, or the fly camera), make a picture, and follow `sessions/lead-fix.md`. Known places to look at first: the tall vertical panels beyond the lake of seed 42 near (-331, 451) (`inbox.md`), the stepped walls at the river mouths (seed 7, river #49), the cave rim (S-186 has it), and the small put-back on seed 7 near (-256, 201) (S-134 has it).

Mikey's first list (2026-11-15, his words in short; he says there are more):

1. **The water does not follow the ground and looks square**, not natural. First idea [unverified]: the water edge follows the quads of the mesh (2 m for lakes, 1 m for rivers), and the shore fade reads the depth only at the vertices (weak spot of S-160). Look at lake and river shores on seeds 1, 7 and 42, on steep and on flat banks. S-181 (Speed) has the idea of a dithered shore edge; do the two rows together or one after the other, because both change `water.gdshader` and the water mesh.
2. **A thing like bright smog deep in the caves.** Mikey is not sure that it is the smog. First ideas [unverified]: the cave fog takes the smog colour too far from a mouth (S-166), or a thing from outside still draws in the deep cave (the smoke wall cards, the sky, dust, mist; the deep-cave cut hides only the terrain). Run the Cave tour, make pictures in the chambers of seeds 1, 7 and 42, and turn the parts off one at a time.
3. **The camera sometimes goes through the ground.** First idea [unverified]: the follow camera reads the heightmap, and a ray only near a cave (S-164), so rocks, crust lids, cave rims and steep ground behind the bear can cut the view (`inbox.md` has two lines on this). Add a recorder probe that counts the frames with the camera below the ground or in a mesh, with the place, so the autoplay finds the places.

Ask Mikey for the other bugs after these three, or read his marks in the next playtest.

Where: Claude Code (local). Model: Opus 5.5, high.

## S-213

**Fix tool: play probes for the camera** (`diagnostics.md`, "Finding bugs"). The recorder checks the camera in each physics tick: below the terrain or the cave floor, inside a collision shape, the near plane cuts the ground, the line from the camera to the bear is blocked for more than 0.5 s. Event `camera_clip` with the place and a picture. A "Camera" section in `report.md`, a count in the autoplay result, and a limit in the GitHub build. Done when the autoplay tour of seeds 1, 7 and 42 finds one or more places where the camera goes through the ground (Mikey sees this bug, S-212), or shows with data that the tour does not go to such places.

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-214

**Fix tool: the world audit** (`diagnostics.md`, "Finding bugs"). A command in `bf-worldgen` (no Godot) that checks rules on the data of a zone and writes `audit.md`: the count and the worst place for each rule. First rules: the distance of the water mesh edge from the true shore line (where the ground is at the water level), water above a low bank, water quads that hang in the air, scatter and obstacles that float, lie under the ground or under water, cave chunk seams and holes, a gap at a mouth rim, a start point on bad ground. Each rule has a limit as data. Run it on seeds 1, 7 and 42 in `cargo test` and on 50 seeds in the nightly GitHub job. Done when the audit shows the square water edge of S-212 as a number.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, high.

## S-215

**Fix tool: the look tour** (`diagnostics.md`, "Finding bugs"). `--bf-test=look-tour`: for each of seeds 1, 7 and 42, select from the world data one view of each kind (steep shore, flat shore, river mouth, rapids, tall fall, cave mouth from outside, tunnel, deep chamber, each obstacle type, the smoke wall, the bear near and far, a wide view). Stop the camera, hide the overlay, wait for the streaming, and save a full 1080p picture, a burst of 5 frames, and a caption file (kind, seed, place, what must be true). Add image numbers to the report: average brightness, and the change between the frames of a burst. `tools/look-tour.sh` runs it on the Intel GPU and makes one contact sheet for each kind. Done when the deep chamber pictures show the bright haze of S-212 with no overlay.

Where: Claude Code (local). Model: Sonnet 5.5, high.

## S-216

**Fix tool: the look review** (`diagnostics.md`, "Finding bugs"). 1) `plan/looks.md`: for each kind of view, what must be true and a list of known faults (square edges, gaps, floating things, wrong fog, holes, z-fighting, popping). 2) A reference set: the pictures that Mikey approved, as `look-reference.zip` on the `game-latest` release (not in the repo). `tools/look-review.sh` compares a new look tour with it and lists the views that changed. 3) The review step: one subagent for each kind of view reads the pictures against `looks.md` and writes the faults with the picture name. Test it: the review must name Mikey's three bugs of S-212 without his words. Write the result, with what it missed, in `diagnostics.md`.

Where: Claude Code (local). Model: Opus 5.5, medium.

## S-217

**Fix tool: the zone fingerprint** (`recipe.md`, risk 1). A function in `bf-worldgen` (no Godot) gives one 64-bit number for a zone: a hash of the bits of the heights, the riverbed and lakebed maps, each water body (kind, level, look, area), the falls, the cave graph (nodes, edges, mouths), the obstacles (after `place_obstacles`), and the scatter instances of the 4 chunks at the start point for each class. Use the settings of the game: size 2048, spacing 1.0, all `Default` params, caves on. Add a test with the stored numbers of seeds 1, 7 and 42, and print each part's number when the test fails, so a session sees which system changed. Add `--fingerprint` to the preview example. A session that changes a plain zone on purpose stores the new numbers and says so in its commit. Add the tool to `tools/README.md`. Done when the test passes on Linux in `tools/check.sh` and in the GitHub build; write in the summary if the Windows build runs it too.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, medium.

## S-218

**Build: the recipe core** (`recipe.md`, "The parts"). Make `rust/worldgen/src/recipe.rs` with `RECIPE_VERSION`, `zone_seed`, `Draw` (our own FNV-1a, `unit`, `signed`, `pick`), `Dial`, `Dials`, `DIAL_TABLE`, `Knob`, `value`, the weight-with-bias helper, `Mode`, `Base`, `ZoneRecipe` with `plain`, `seeded` and `describe`. Add `LookParams` (empty for now). In each system file add `pub const KNOBS: &[Knob] = &[]`, `pub fn seeded(base, dials, draw)` that gives `base` back, and `pub fn fix(&mut params)` that does nothing. Tests: the same name gives the same number; two names give different numbers; `value` with dials 0 and no wobble gives `base` exactly; `value` stays in `lo` to `hi` for 10,000 draws; over 10,000 seeds the hero counts are near the weights; `plain` gives the base structs. Do not change how a zone is made: the fingerprint of S-217 must pass.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, high.

## S-219

**Build: the game uses one recipe** (`recipe.md`, risk 3). `BfTerrain::regenerate` makes one `ZoneRecipe` from its exports (the `Base`) and gives it to `generate`, the caves, the obstacles, and through a `#[func]` to `BfScatter` and `BfSmokeEdge`. No node calls `XParams::default()` for a zone. `BfSmokeEdge` keeps its exports, but they go into the `Base` of the terrain, so the caves and the smoke edge read the same `SmokeParams`. Add `--bf-recipe=plain|seeded` (default `plain`) and one dev menu line. Show `describe()` in the dev menu, in `session.json` and at the top of `report.md`, with `RECIPE_VERSION`. Done when the fingerprint passes, the game loads headless, and a test shows that a changed smoke front moves the cave margin.

Where: Claude Code (local). Model: Sonnet 5.5, high.

## S-220

**Build: the water recipe** (`recipe.md`, "What each system row does"). System: `worldgen/water.rs`, `rivers.rs`, `waterfalls.rs`. Dial: `wet`; `strange` for the looks. Move the look weights of `pick_look`, `RIVER_LOOK_WEIGHTS` and the look tables (colour, clarity, density, viscosity) into data in `WaterParams`. Knobs, as a start: `lake_part_min`, `full_lake_chance`, `pond_part`, `wet_min_area_m2`, `river_part_min`, `river_part_max`, `flow_max_mps`. The bias of the look weights for `strange`: more milky, rust, mud and tar. Keep `bank_margin_m`, `min_depth_m` and the fall limits fixed.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, medium.

## S-221

**Build: the terrain recipe** (`recipe.md`, "What each system row does"). System: `worldgen/highlands.rs`. Dial: `rugged`; `strange` for the ground colours. Knobs, as a start: `relief_m`, `ridge_wavelength_m`, `ridge_sharpness`, `valley_at`, `warp_amount_m`, `massif_amount`, `erosion.strength`, `floor_height_m`. Move the 6 ground colours (`highlands.rs:661`) into `LookParams` as one palette; add two more palettes with weights and a bias for `strange`. Keep the spacing, the size, `max_bend_m` and the drainage step fixed. Check with the start point and the cave tests that a zone at each end still has a start point and a hillside for a mouth.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, high.

## S-222

**Build: the cave recipe** (`recipe.md`, "What each system row does"). System: `worldgen/caves/`. Dial: `hollow`; `strange` for the wall noise. Move the ranges that are numbers in `build.rs` (lines 462 to 593 and 760 to 763) into `CaveParams`. Do not add a draw to the old counter `Rng`: a new seeded value uses `Draw` by name. Knobs, as a start: `systems_min`, `systems_max`, `depth_max_m`, `chamber_width_max_m`, `chamber_height_max_m`, `branch_max`, `loop_chance`, `sinkhole_chance`, `chamber_noise_part`, `tunnel_noise_part`, `bend_m`. Keep `roof_min_m`, `walk_slope`, `edge_margin_m`, `start_clear_m` and the grid fixed. Both ends must pass the tests of `caves/tests.rs`.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, high.

## S-223

**Build: the obstacle recipe** (`recipe.md`, "What each system row does", risk 10). System: `worldgen/obstacles.rs`. Dial: `danger`. Move the period and size formulas (lines 793 to 830) into data in `ObstacleParams`. Make `max_count` a weight table with a seeded mix: each zone has two or three main types (`Draw::pick`), and the other types are rare. Knobs: `density`, `hard_at_m`, `near_density`. Keep the tells of `game/obstacles.rs`, `clear_start_m`, `first_within_m` and the gaps fixed (fairness). The first obstacle of each main type near the start stays easy.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, medium.

## S-224

**Build: the scatter recipe and seeded prop shapes** (`recipe.md`, "What each system row does", risk 6). System: `worldgen/scatter.rs`, `props.rs`, `game/scatter.rs`. Dial: `cover`; `strange` for the shapes. Knobs: the `chance` and `size_max_m` of each class. `rock_mesh`, `pebble_mesh` and `shrub_mesh` take shape values from the recipe (bump, squash, cut, crown size), so each zone has its own rocks and shrubs. The number of variants and the triangles of each mesh stay the same, so the cost does not change. `BfScatter` makes its meshes again when the zone changes.

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-225

**Build: the recipe for smog, sky and the smoke edge** (`recipe.md`, "What each system row does", risk 6). System: `game/atmosphere.rs`, `worldgen/smoke.rs`, `game/smoke_edge.rs`, the cave rock colour of `game/caves.rs`. Dial: `smog`; `strange` for the colours. Fill `LookParams`: fog colour and density, smog height, sky colours, sun colour, sun elevation, smoke colour, cave rock colour. `BfAtmosphere` sets these one time when the zone changes, not each frame. Knobs of the smoke edge: `swing_fraction`, `height_max_m`, the lobe list. The thin end of the fog density is fixed by speed: use the value of today as `lo` until S-228 gives a number. The front speed stays below the walk speed.

Where: Claude Code (local). Model: Sonnet 5.5, medium.

## S-226

**Build: traits and promises** (`recipe.md`, "Traits" and "Limits"). Add the trait table, the promise check after the zone is made, the tries (`attempt`, 3 at most), the fallback with no trait, and the log line for each failed promise. The first trait is "the great lake": it needs `wet` above 0.3, it sets the water knobs, and its promise is one lake of 40,000 m² or more. Show the trait in `describe()`. The audit (S-227) must show that fewer than 10% of 50 seeds need a second try. If not, change the trait so that it builds its lake on purpose.

Where: Claude Code (local). Model: Opus 5.5, high.

## S-227

**Fix tool: the audit checks recipes** (`recipe.md`, "How the world audit checks a recipe"). Add to the world audit of S-214: level 1 (10,000 recipes: ranges, `fix` rules, hero counts, a table for each dial), level 2 (the seeded zones of seeds 1, 7 and 42 in `cargo test` and of 50 seeds each night: shape rules, promises, time to make the zone), level 3 (corners: each knob at `lo` and at `hi`, a part of the list each night). Write the "Recipes" section of `audit.md`. Done when the audit runs with the empty knob tables of S-218 and fails in a test that gives it a knob with a range that breaks a rule.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, high.

## S-228

**Speed: cost numbers and the corner test** (`recipe.md`, "The speed budget"). Add `--bf-recipe=corner:<system>`. For each system that has knobs, run the Benchmark and its tour with the corner recipe on the Intel GPU, one process for each. Add the cost numbers to the audit (water triangles in view, fall area, props in one chunk ring, chamber triangles, transparent layers) and set each limit from the measured views. If a corner is over its target, make the range smaller and write the reason in the knob table. Write the numbers in `play-notes.md`.

Where: Claude Code (local, Mikey's laptop). Model: Sonnet 5.5, medium.

## S-229

**Mikey plays: seeded zones.** Mikey turns on "Recipe: seeded" in the dev menu and goes through five seeds (F4), about two minutes each. He says which zones feel different, which feel wrong, and which hero he likes. He also looks at the contact sheet of 12 zones. Then he answers the four questions of `recipe.md`. A Brain session writes his answers in `decisions.md` and changes the tables.

Where: Mikey's laptop.

## S-231

**Tool: the workflow cost** (R29). Make `tools/workflow-cost.py` (no packages). It reports, for each week: the share of commits that change only docs, the words that a lead reads before work (`CLAUDE.md`, `HANDOFF.md`, `STATE.md`, `decisions.md`, `tech.md` and the longest brief), and the size of `STATE.md` and `rows.md`. Add it to `tools/README.md`. The Bird's-eye lead runs it. If a number climbs two weeks in a row, it adds a Hygiene row. The numbers on 2026-11-18: 69 of 136 commits change only docs (50%), about 8,300 words before work, `STATE.md` 90 of 90 lines, `rows.md` 5,333 words.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, low.

## S-232

**Research: sort the Broken Friends topics of the wanted list** (R26). The wiki's `Background/Wanted articles.md` has 53 B topics, and none has an article. The game changed direction since the list was made (the 3D restart, demo 2, `recipe.md`). Compare each B topic with `vision.md`, the specs and `recipe.md`. Mark it keep, rewrite or drop. Rewrite the kept topics for the game of today, and delete the topics that the wiki does not need. Do not change the D topics: the Driftloom part of the wiki becomes a Ballad Falls folder at the rename. Follow the wiki's `CLAUDE.md` and run `python3 _tools/check.py` there before you push.

Where: Claude Code (local or cloud). Model: Sonnet 5.5, medium.
