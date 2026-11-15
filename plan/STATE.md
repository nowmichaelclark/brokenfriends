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
| S-162 | **Build: rivers with water** (`demo-2.md`, "Water bodies"). The second (wet) flow accumulation, river surfaces that never rise, river meshes with flow in the vertices, two ripple layers, rapids, damp banks. Add the river view to the Water view test. Parallel: S-161, S-164. | Claude Code (local) | Opus 5.5, high | **ready** |
| S-163 | **Build: swimming and wet fur** (`demo-2.md`, "Swimming"). Replace the float of S-160: wade, swim, float line, viscosity, current, stamina, bank exit, climb from the water, wet parts that dry. Recorder events `swim_*`, `bank_*`. Parallel: S-161, S-162, S-164. | Claude Code (local) | Sonnet 5.5, high | **ready** |
| S-164 | **Build: the first walkable cave** (`demo-2.md`, "Walkable caves"). Chunk meshes on threads, collision near the bear, holes and rim meshes at mouths, the put-back, ground event, camera, start point, scatter and obstacles made cave-aware. Add the Cave tour test and the `cave` samples. Parallel: S-162, S-163, S-165. **Fable 5.2 pick:** it touches the most systems (Terrain3D, threads, Jolt, bear, camera, recorder), and its bugs (falls through the floor, hitches) are the hardest to find later. | Claude Code (local) | Opus 5.5, high | **ready** |
| S-165 | **Build: waterfalls** (`demo-2.md`, "Waterfalls"). Falls at river steps and lake outlets, one tall and one wide feature fall per zone, curtain, foam, splash, mist, the push on the bear. Add the fall views to the Water view test. Parallel: S-164, S-166. | Claude Code (local) | Opus 5.5, high | waiting on S-162 |
| S-166 | **Build: cave light** (`demo-2.md`, "Cave light"). Sky visibility in `AO`, the cave fog, glowing minerals, the uniform on the fur and the water, the deep-cave cut (on and off in the Cave tour). Parallel: S-163, S-165. | Claude Code (local) | Sonnet 5.5, high | waiting on S-164 |
| S-167 | **Play: the first look at water and caves.** Download the newest zip. Start it with `--gpu-index 0`. Run the Water view, the Cave tour and the Benchmark, then play for 10 minutes. Send the zip and one line each on: the water colours, the swim, the first cave, the questions in `demo-2.md`. | Mikey's TUF laptop | Mikey | waiting on S-160, S-163, S-164 |
| S-168 | **Build: pools in caves** (`demo-2.md`, "Pools in caves"). Water at the low points of chamber floors, with the cave light. Parallel: S-169 to S-172. | Lead chat (cloud worker) | Sonnet 5.5, medium | waiting on S-160, S-166 |
| S-169 | **Build: the far walls of big chambers.** A second level of cave chunks (2 m voxels from about 48 m), hidden seams, and occluders from the cave shells if the Cave tour shows overdraw. Parallel: the water rows. | Claude Code (local) | Opus 5.5, high | waiting on S-164 |
| S-170 | **Build: the ragdoll, lost limbs and swim gaits in water** (R11, R12, R18). Lift and drag for each ragdoll part, floating lost limbs, a dog paddle from the limbs that the bear still has. Parallel: the cave rows. | Lead chat (cloud worker) | Sonnet 5.5, high | waiting on S-163 |
| S-171 | **Build: autoplay for water and caves** (`diagnostics.md`, "Autoplay"). Tour tasks: swim across a lake, get out at a bank, go into the nearest mouth, reach a chamber, stand in a fall. Caves in the nightly wander. Parallel: S-168 to S-170. | Lead chat (cloud worker) | Sonnet 5.5, medium | waiting on S-163, S-164 |
| S-172 | **Build: obstacles with water and caves** (R23). No obstacles in water or hole cells, a crumbling floor that drops the bear into a cave, stepping stones across rivers. Parallel: S-168 to S-171. | Lead chat (cloud worker) | Sonnet 5.5, high | waiting on S-132, S-162, S-164 |
| S-173 | **Play: the demo 2 playtest.** The same as S-167, with the waterfalls, the cave light, the pools and the far walls. Play for 10 minutes and send the zip and notes. | Mikey's TUF laptop | Mikey | waiting on S-165, S-166, S-168 to S-172 |
| S-174 | **Brain: tune demo 2 from S-167.** Read the zip and Mikey's answers. Write his decisions with him, and add tuning rows S-175 to S-179. | Claude Code (local) | Opus 5.5, medium | waiting on S-167 |
| S-150 | **Build: tune the obstacles again from the next playtest.** Read "Obstacles" in `report.md`. Check the rolling stones (hits per tell; the lane is now 1 to 2.5 m wide), the ash crust (falls against escapes; the shake is now 1.8 s) and the launch vents (the events end with `:dy=` and `:off=`: if a launch still misses, read them, and look at the marks for a bear that hits the face). Then change the defaults on `Obstacles` and `SimParams::default()`. Also check the new steer after a launch (`launch_steer_s`): the bear must not feel stuck on the wall or lose the grab at the ledge lip. | Lead chat (cloud worker) | Sonnet 5.5, medium | waiting on Mikey's play (S-137) |
| S-180 | **Build: tune the still water from the Intel GPU numbers** (`demo-2.md`, "The GPU budget"). Read the Water view results of the lead (`--bf-test=water-view`, with and without `--bf-water=off`, each in a new process) and Mikey's notes on the water colours (S-167). If the lake view costs more than 1.0 ms of GPU time, use the cuts on `Water` in order (`ripple_layers` 1, `shore_fade_m` 0, `receive_shadows` off, `shaded` off). Change the defaults of `BfWater` and `WaterParams`. | Claude Code (local) | Sonnet 5.5, medium | waiting on the Water view on the Intel GPU |

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
