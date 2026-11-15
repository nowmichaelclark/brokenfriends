# State

Updated: 2026-11-15 (Brain S-142: demo 2 plan, `demo-2.md`, rows S-160 to S-174)

Write this file again each time. Do not add to the end (90 lines at most).

## Phase

**Demo 1** (`demo-1.md`): walk, jump and climb through one biome under smog. Rust makes a zone of scorched highlands; Terrain3D shows it
(`game/README.md`). The bear walks, jumps and climbs (stamina, grip). Smog, sun, sky and dust are built (R19). Rust makes the bear from SDF parts
(R17) and poses them (R18): planted feet, springs, hands that hold the wall, and a gait from the limbs it has. A wall of dark smoke surrounds the zone
(R14) and kills a bear that stays in it (R16). Nobody has seen the smog, the smoke, the animation or the scatter in the Mobile renderer yet. The
playtest recorder (R22) writes one zip for each session. A dev menu (hold Back, or F10) has the dev tools and tests. Each zone has about 340 seeded
obstacles (R23): 8 types, and (S-129) thorn snags, rock teeth and falling rocks, which tear seams. The draw distance follows the smog (767 m). An
autoplay bot runs a tour in each game build and wanders each night. A hard hit makes the bear tumble as a Jolt ragdoll (R11); hits tear the seams, a
limb dangles or falls off, and the bear sews it on again (R12). Rocks, pebbles and dead shrubs (S-108) stream in near the camera. The S-125 freezes
come from the desktop (`research/freezes.md`). On Intel UHD Graphics, a frame needs about 12 to 13.4 ms of GPU time with the scatter (S-139: light
terrain shader, cheaper shadows and sky light, `research/gpu-speed.md`). The GitHub build ae685ec gives 70 fps on the Intel GPU in the tests and in the autoplay tour; the first run has a slow warm-up.

## Agenda

To start a session, Mikey opens the place in the "Where" column. He selects the model and effort, then types "Run S-NNN". At the end of a session, the session deletes its row. It changes the rows that it unblocks to **ready**. It adds follow-up rows at the bottom.
Next number: S-144. A cloud worker of the lead Brain chat numbers its new rows from the range in its brief.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-124 | **Build: playtest replay** (`diagnostics.md`, "Later: replay"). Record the input of each physics tick in the playtest. A headless command replays a playtest with its seed and compares the bear's path with the recorded positions. | Claude Code | Opus 5.5, high | **ready** |
| S-126 | **Build: tune the walk animation from the playtest.** Read Mikey's notes and `anim_us` in `samples.jsonl` of the playtest. Change the defaults on `BfBearModel` (stride, bounce, waddle, springs) and keep `AnimParams::default()` in `anim.rs` the same. Fix what looks wrong at the marks. | Claude Code | Sonnet 5.5, medium | waiting on Mikey's feel notes (S-137) |
| S-130 | **Build: tune the climbing hands and the gaits from the playtest** (R18). Read Mikey's notes and the `limb_off`, `gait` and `grab` events. Change the defaults of the holds (`hold_step_m`, `hand_reach`, `hang_*`) and of the gaits (hop, lie, crawl, the speed scales on `Bear`). Keep `AnimParams::default()` and `GaitTuning::default()` the same as the exported defaults. | Claude Code | Sonnet 5.5, medium | waiting on Mikey's feel notes (S-137) |
| S-131 | **Build: tune the smoke edge from the playtest** (R14, `demo-1.md`). Read Mikey's notes, `smoke_m` and `smoke_s` in `samples.jsonl`, and the events `smoke_enter`, `smoke_leave` and `death:smoke:*` with their shots. Change the defaults on `BfSmokeEdge` (limits, wall speed, opacity, haze, card count). Check the frame time near the wall (overdraw) and cut cards if it is high. | Claude Code | Sonnet 5.5, medium | waiting on Mikey's feel notes (S-137) |
| S-137 | **Play: the feel test on the Intel GPU of the TUF.** Download the newest zip with `tools/get-build.sh`. Start it with `--gpu-index 0`. The lead Brain already ran the three tests (`play-notes.md`, 2026-11-15). Play for 10 minutes. Send the zip, and one line each on: the new, cheaper look of the ground (S-139); the rocks, pebbles and dead shrubs (S-108), and if they pop in; the walk; the climb; the smoke wall. | Mikey's TUF laptop | Mikey | **ready** |
| S-109 | **Brain** with Mikey: play the full demo 1, decide what to fix. Demo 2 planning moved to S-142. | Project chat | Opus 5.5, medium | waiting on S-137 |
| S-134 | **Build: fix what the first nightly autoplay finds** (`diagnostics.md`, "Autoplay"). Read `autoplay-nightly.md` on the `game-latest` release and the "Autoplay" notes of the game builds. Fix the bugs that it shows (falls through the ground, errors, many stuck or climb loop events at one place). If a task of the tour is often "missed" on a seed, make the bot better (`playtest/autoplay.rs`). Also look at the small put-back (0.26 m) on seed 7 at the stepping stones near (-256, 201): the collision and the heightmap do not agree there. | Claude Code | Sonnet 5.5, medium | waiting on the first nightly run (after the push of S-133) |
| S-141 | **Build: tune the scatter from the playtest** (`research/scatter.md`). Read Mikey's notes, the overlay line `scatter:` in the marks, and the frame time with the scatter on and off (dev menu). Change the defaults on `BfScatter` (density, size scale, ranges, collision) and `ScatterParams::default()` with them. If the draw calls or the chunk build time are too high on the Intel GPU, use the ideas in `inbox.md`. Also find the hitches of the Benchmark on seed 1 (one 400 ms frame on build ae685ec), the slow first 10 to 20 s of the first run (GPU 18 to 22 ms, not 12 ms), and the shader compiles in the first minute of play (`play-notes.md`, 2026-11-15). | Claude Code | Sonnet 5.5, medium | waiting on Mikey's play (S-137) |
| S-143 | **Play: the integrated-graphics test on the Vivobook**, the same as S-137, when Mikey has the Vivobook again. | Mikey's Vivobook | Mikey | waiting on the Vivobook |
| S-155 | **Build: check the S-135 tear and tumble values from the next playtest.** Count the lost legs and the time on the ground in `report.md`. If legs still come off more than about 3 times in 30 minutes, lower `seam_damage_per_speed` again. Add a recorder line for the speed of each limb hit, so the next tune has data. | Lead chat (cloud worker) | Sonnet 5.5, medium | blocked on the next playtest |
| S-163 | **Build: swimming and wet fur** (`demo-2.md`, "Swimming"). Replace the float of S-160: wade, swim, float line, viscosity, current, stamina, bank exit, climb from the water, wet parts that dry. Recorder events `swim_*`, `bank_*`. Parallel: S-161, S-162, S-164. | Claude Code (local) | Sonnet 5.5, high | **ready** |
| S-167 | **Play: the first look at water and caves.** Download the newest zip. Start it with `--gpu-index 0`. Run the Water view, the Cave tour and the Benchmark, then play for 10 minutes. Send the zip and one line each on: the water colours, the swim, the first cave, the questions in `demo-2.md`. | Mikey's TUF laptop | Mikey | waiting on S-160, S-163, S-164 |
| S-168 | **Build: pools in caves** (`demo-2.md`, "Pools in caves"). Water at the low points of chamber floors, with the cave light. Parallel: S-169 to S-172. | Lead chat (cloud worker) | Sonnet 5.5, medium | **ready** |
| S-169 | **Build: the far walls of big chambers.** A second level of cave chunks (2 m voxels from about 48 m), hidden seams, and occluders from the cave shells if the Cave tour shows overdraw. Parallel: the water rows. | Claude Code (local) | Opus 5.5, high | **ready** |
| S-170 | **Build: the ragdoll, lost limbs and swim gaits in water** (R11, R12, R18). Lift and drag for each ragdoll part, floating lost limbs, a dog paddle from the limbs that the bear still has. Parallel: the cave rows. | Lead chat (cloud worker) | Sonnet 5.5, high | waiting on S-163 |
| S-171 | **Build: autoplay for water and caves** (`diagnostics.md`, "Autoplay"). Tour tasks: swim across a lake, get out at a bank, go into the nearest mouth, reach a chamber, stand in a fall. Caves in the nightly wander. Parallel: S-168 to S-170. | Lead chat (cloud worker) | Sonnet 5.5, medium | waiting on S-163, S-164 |
| S-172 | **Build: obstacles with water and caves** (R23). No obstacles in water or hole cells, a crumbling floor that drops the bear into a cave, stepping stones across rivers. Parallel: S-168 to S-171. | Lead chat (cloud worker) | Sonnet 5.5, high | waiting on S-132, S-162, S-164 |
| S-173 | **Play: the demo 2 playtest.** The same as S-167, with the waterfalls, the cave light, the pools and the far walls. Play for 10 minutes and send the zip and notes. | Mikey's TUF laptop | Mikey | waiting on S-165, S-166, S-168 to S-172 |
| S-174 | **Brain: tune demo 2 from S-167.** Read the zip and Mikey's answers. Write his decisions with him, and add tuning rows S-175 to S-179. | Claude Code (local) | Opus 5.5, medium | waiting on S-167 |
| S-150 | **Build: tune the obstacles again from the next playtest.** Read "Obstacles" in `report.md`. Check the rolling stones (hits per tell; the lane is now 1 to 2.5 m wide), the ash crust (falls against escapes; the shake is now 1.8 s) and the launch vents (the events end with `:dy=` and `:off=`: if a launch still misses, read them, and look at the marks for a bear that hits the face). Then change the defaults on `Obstacles` and `SimParams::default()`. Also check the new steer after a launch (`launch_steer_s`): the bear must not feel stuck on the wall or lose the grab at the ledge lip. | Lead chat (cloud worker) | Sonnet 5.5, medium | waiting on Mikey's play (S-137) |
| S-181 | **Build: cut the water to 1.0 ms or less.** After S-180 the lake view costs about 1.2 ms (target 1.0). Test these in new processes with `--bf-water-<name>=<value>`: the smog fog of the water with a cheap own formula (fog costs about 0.5 ms), a dithered shore edge instead of blended shore quads (no blending at all), and fewer far quads (a 8 m quad from 150 m). Keep the look. | Claude Code (local) | Sonnet 5.5, medium | **ready** |
| S-182 | **Build: a water sweep script.** `tools/water-sweep.sh` runs the Water view once for each setting in a list, each in a new process, alternates "water off" between them, and writes one table of GPU ms (average of the 3 seeds, off subtracted). The noise of one run is about 0.5 ms, so each setting runs 3 times. | Claude Code (local) | Sonnet 5.5, low | **ready** |
| S-183 | **Build: why a lake view without water costs 14.5 ms.** The Water view without water is 2 ms above the Benchmark (12.5 ms). Find the cause (the bed under the lake, the open view, the far smog) with probes. Idea: draw the opaque water before the terrain, so that the bed under it is not shaded. | Claude Code (local) | Sonnet 5.5, medium | **ready** |
| S-184 | **Play: look at the new water.** Compare the new water (own light, opaque deep quads, no shadows on the water) with the old look: colour, ripples, sun glint, the bed through clear water. Ask Mikey only about feel and taste. Add this to S-167 if S-167 has not run yet. | Mikey | – | **ready** |
| S-185 | **Play: the Cave tour on the Intel GPU** (lead Brain, local). Run `--gpu-index 0 -- --bf-test=cave-tour --bf-record`, then again with `--bf-caves=off`, each in a new process. Write the GPU ms, the draw calls and the cave CPU ms of each part into `play-notes.md`. If the chamber is over 1.5 ms or the outside part over 0.3 ms, add a row for the cuts of `demo-2.md`. | Claude Code (local) | Opus 5.5, medium | **ready** |
| S-186 | **Build: the cave rim looks like the ground.** Look at the mouths of seeds 1, 7 and 42 in the Mobile renderer: the hole edge, a step at the terrain edge, the colour and the grain. Fix the seams that show (`rim_ring_cells`, `rim_sink_m`, the colour). | Claude Code (local) | Sonnet 5.5, medium | **ready** |
| S-187 | **Build: a dev menu line for the caves** (on and off) and a "go to the nearest mouth" teleport, so Mikey finds a cave without a map. | Lead chat (cloud worker) | Sonnet 5.5, low | **ready** |
| S-190 | **Brain: the river view on the Intel GPU.** Run the Water view of the S-162 build two times, each in a new process: `~/Games/broken-friends-linux/BrokenFriends.x86_64 --gpu-index 0 -- --bf-test=water-view`, then the same with `--bf-water=off` at the end. Write the GPU ms of the view "a river with rapids" for seeds 1, 7 and 42 into `play-notes.md`. If the river costs more than the lake view, add a row to cut it. | Claude Code (local) | Opus 5.5, medium | **ready** |
| S-191 | **Build: longer rivers, if Mikey wants them** (S-162 "Assumed" in `game/README.md`). The rivers are short, steep streams because closed lakes catch most of the rain. Try more full lakes (`full_lake_chance`), a lower `wet_min_area_m2` or wider valley floors, and compare the river length and the rapids of seeds 1, 7 and 42 (`print_rivers_for_wet_areas`). | Claude Code (local) | Sonnet 5.5, medium | waiting on Mikey's answer (S-167) |
| S-192 | **Build: tune the river look from the playtest.** Read Mikey's notes on the rivers of S-167: the foam (about half of the cells are rapids), the flow speed of the ripples, the step where a river meets a lake. Change the defaults on `BfWater` (`foam_strength`, `flow_cycle_s`, `flow_ripple_gain`) and in `WaterParams` (`rapid_drop_m`, `mouth_slope`). | Claude Code (local) | Sonnet 5.5, medium | waiting on S-167 |
| S-195 | **Build: the cave light on the water.** Read the global uniform `bf_cave_sky` (`caves.rs`) in `water.gdshader`: use it for the water `AO` and the water fog, so a pool in a cave is dark. S-166 left the water files alone because S-162 changed them. S-168 needs this. Parallel: S-168. | Claude Code (local) | Sonnet 5.5, low | **ready** |
| S-196 | **Brain: the Cave tour with the cut on and off.** On the Intel GPU, run `-- --bf-test=cave-tour` and `-- --bf-test=cave-tour --bf-cave-cut=off`, each in a new process, 3 times each. Write the GPU ms of the tunnel and chamber parts, and the cost of the new cave fog and AO. Read the screenshots: change `dark_level`, the fog and the minerals only if they look wrong. | Claude Code (local) | Sonnet 5.5, medium | **ready** |
| S-197 | **Build: a stronger deep-cave cut.** The cut hides only the terrain and the sun shadows. Also hide the scatter, the obstacles and the water meshes while it is on, and show them again with the terrain. Measure on and off. Only if S-196 shows that the cut saves too little. | Claude Code (local) | Sonnet 5.5, medium | waiting on S-196 |
| S-200 | **Play: the falls on the Intel GPU** (lead Brain, local). Run the Water view on `--gpu-index 0` with the new views (`tallest_fall`, `widest_fall`), each in a new process: `-- --bf-test=water-view`, then `--bf-waterfalls=off`, then `--bf-waterfalls-mist=off`, then `--bf-waterfalls-splash=off`, then `--bf-water=off`. Write the GPU ms and the draw calls of each fall view into `play-notes.md`. If a fall costs more than 0.8 ms, make the cuts of `demo-2.md` in order. | Claude Code (local) | Opus 5.5, medium | **ready** |
| S-201 | **Build: look at the falls and tune them.** Look at the tall and the wide fall of seeds 1, 7 and 42 in the Mobile renderer: the jagged edge where the sheet meets the ground, the colour of the curtain, the splash cards, the mist. Fix what shows (`ground_offset_m`, the mask fade in `waterfall_mesh.rs`, `streak_*`, `foam_*`). | Claude Code (local) | Sonnet 5.5, medium | **ready** |
| S-202 | **Build: true cliffs for the feature falls, if Mikey wants them** (S-165 "Assumed" in `game/README.md`). Carve a flat lip and a near-vertical drop with an overhang at the best candidate, and raise the river levels to match. Check that caves, obstacles and scatter still fit the carved ground. | Claude Code (local) | Opus 5.5, high | waiting on Mikey's answer (S-167) |
| S-203 | **Build: the bear in a fall** (R11, R12, R18). Tune `fall_push_accel` from the recorder events `fall_in` and `fall_out`. Add the tumble, the wet parts and the splash on the bear when it stands in a curtain, and a spray when it enters the pool. | Lead chat (cloud worker) | Sonnet 5.5, medium | waiting on S-163, S-167 |
| S-204 | **Build: falls in the autoplay** (`diagnostics.md`, "Autoplay"). Add the tour task "stand in a fall" (go to the foot of the tallest fall within 580 m, then climb into the curtain). Count `fall_in` events in the report. | Lead chat (cloud worker) | Sonnet 5.5, low | waiting on S-171 |

## Combined runs

A combined run does more than one row in one session. Use it when a strong model is available for a short time.

1. Do the rows in the order that the row gives.
2. Finish each row fully before you start the next row. This includes
   the tests, `STATE.md`, the commit and the push. Each push is a save
   point.
3. Keep the context small. Do not read `archive/`. To read external
   docs, use a subagent. Tell it to send back only a short summary.
4. If the context becomes full, finish the current row, push, and
   stop.
5. At the end, delete the row of the combined run.

The cloud cannot test the Mobile look or the speed. Mikey's play rows do.

## How Mikey plays (decision R10)

After each push to `main` that changes `rust/`, `game/` or `tools/`,
GitHub builds the game for Linux and Windows. The zips go on the
`game-latest` release. Mikey downloads a zip and plays. He does not
install Rust or the Godot editor. He tells Claude what he feels and
sees, and Claude sessions change the values.

## How we chose the models

- **Opus 5.5, high:** for rows with many unknowns or difficult maths.
  These are the first setup (all later rows use it), terrain
  generation and climbing.
- **Sonnet 5.5:** for usual building work, such as a character
  controller, fog or instancing. Use high effort for the first code in
  a new area. Use medium effort when the work is mostly settings.
- **Fable 5.2:** limited: combined runs on the hardest rows. If the plan limit is low, use Sonnet 5.5, high, for an Opus row.

## Blockers

None. The questions in `demo-1.md` are for Mikey. The build does not wait.
