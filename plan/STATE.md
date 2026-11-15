# State

Updated: 2026-11-15 (Brain: demo 2 starts now, R24 to R26; the lead Brain chat runs cloud workers)

Write this file again each time. Do not add to the end (90 lines at most).

## Phase

**Demo 1** (`demo-1.md`): walk, jump and climb through one biome
under smog. Rust makes a zone of scorched highlands; Terrain3D shows it
(`game/README.md`). The bear walks, jumps and climbs (stamina, grip).
Smog, sun, sky and dust are built (R19). Rust makes the bear from SDF
parts (R17) and poses them (R18): planted feet, springs, hands that
hold the wall, and a gait from the limbs it has. A wall of dark smoke
surrounds the zone (R14) and kills a bear that stays in it (R16).
Nobody has seen the smog, the smoke, the animation or the scatter in
the Mobile renderer yet. The playtest recorder (R22) writes one zip for each
session. A dev menu (hold Back, or F10) has the dev tools and tests.
Each zone has about 340 seeded obstacles (R23): 8 types, and (S-129)
thorn snags, rock teeth and falling rocks, which tear seams. The draw distance follows the smog
(767 m). An autoplay bot runs a tour in each game build and wanders
each night. A hard hit makes the bear tumble as a Jolt ragdoll (R11);
hits tear the seams, a limb dangles or falls off, and the bear sews it
on again (R12). Rocks, pebbles and dead shrubs (S-108) stream in near
the camera. The S-125 freezes come from the desktop (`research/freezes.md`).
On Intel UHD Graphics, a frame needs about 12 to 13.4 ms of GPU time
with the scatter (S-139: light terrain shader, cheaper shadows and sky
light, `research/gpu-speed.md`). Seed 1 had scatter-time hitches.

## Agenda

To start a session, Mikey opens the place in the "Where" column. He
selects the model and effort, then types "Run S-NNN". At the end of a
session, the session deletes its row. It changes the rows that it
unblocks to **ready**. It adds follow-up rows at the bottom.
Next number: S-144. A cloud worker of the lead Brain chat numbers its
new rows from the range in its brief.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-124 | **Build: playtest replay** (`diagnostics.md`, "Later: replay"). Record the input of each physics tick in the playtest. A headless command replays a playtest with its seed and compares the bear's path with the recorded positions. | Claude Code | Opus 5.5, high | **ready** |
| S-126 | **Build: tune the walk animation from the playtest.** Read Mikey's notes and `anim_us` in `samples.jsonl` of the playtest. Change the defaults on `BfBearModel` (stride, bounce, waddle, springs) and keep `AnimParams::default()` in `anim.rs` the same. Fix what looks wrong at the marks. | Claude Code | Sonnet 5.5, medium | waiting on Mikey's feel notes (S-137) |
| S-130 | **Build: tune the climbing hands and the gaits from the playtest** (R18). Read Mikey's notes and the `limb_off`, `gait` and `grab` events. Change the defaults of the holds (`hold_step_m`, `hand_reach`, `hang_*`) and of the gaits (hop, lie, crawl, the speed scales on `Bear`). Keep `AnimParams::default()` and `GaitTuning::default()` the same as the exported defaults. | Claude Code | Sonnet 5.5, medium | waiting on Mikey's feel notes (S-137) |
| S-131 | **Build: tune the smoke edge from the playtest** (R14, `demo-1.md`). Read Mikey's notes, `smoke_m` and `smoke_s` in `samples.jsonl`, and the events `smoke_enter`, `smoke_leave` and `death:smoke:*` with their shots. Change the defaults on `BfSmokeEdge` (limits, wall speed, opacity, haze, card count). Check the frame time near the wall (overdraw) and cut cards if it is high. | Claude Code | Sonnet 5.5, medium | waiting on Mikey's feel notes (S-137) |
| S-132 | **Build: tune the obstacles from the playtest.** Read Mikey's notes and "Obstacles" in `report.md` (hits, falls, escapes and launch results by type). Change the defaults on `Obstacles` and `Terrain`, and `SimParams::default()` with them. Include the stepping stones, updrafts and sinking sand of S-128, and the thorn snags, rock teeth and falling rocks of S-129 (`thorn_*`, `teeth_*`, `rock_*`; events `snag`, `tear`, `pin`, `free`). Check that the dust and steam show in the Mobile renderer (marks and `shots/`). | Lead chat (cloud worker) | Sonnet 5.5, medium | **in progress** |
| S-137 | **Play: the feel test on the Intel GPU of the TUF.** Download the newest zip with `tools/get-build.sh`. Start it with `--gpu-index 0`. Run the Benchmark, the Draw distance test and the Autoplay tour from the Tests page, then play for 10 minutes. Send the zip, and one line each on: the new, cheaper look of the ground (S-139); the rocks, pebbles and dead shrubs (S-108), and if they pop in; the walk; the climb; the smoke wall. | Mikey's TUF laptop | Mikey | **ready** |
| S-109 | **Brain** with Mikey: play the full demo 1, decide what to fix. Demo 2 planning moved to S-142. | Project chat | Opus 5.5, medium | waiting on S-137 |
| S-134 | **Build: fix what the first nightly autoplay finds** (`diagnostics.md`, "Autoplay"). Read `autoplay-nightly.md` on the `game-latest` release and the "Autoplay" notes of the game builds. Fix the bugs that it shows (falls through the ground, errors, many stuck or climb loop events at one place). If a task of the tour is often "missed" on a seed, make the bot better (`playtest/autoplay.rs`). Also look at the small put-back (0.26 m) on seed 7 at the stepping stones near (-256, 201): the collision and the heightmap do not agree there. | Claude Code | Sonnet 5.5, medium | waiting on the first nightly run (after the push of S-133) |
| S-135 | **Build: tune the tumble and the seams from the playtest** (R11, R12). Read Mikey's notes and "Tumbles and limbs" in `report.md` (causes, speeds, time on the ground, torn seams, lost and sewn limbs), and `seam_min` in `samples.jsonl`. Change the defaults on `Bear` (`tumble_*`, `seam_*`, `thread_wear_per_m`, `sew_*`) and keep `TumbleParams::default()` and `TearParams::default()` the same. Look at the marks for a ragdoll that shakes, goes through the ground or stands up wrong. | Lead chat (cloud worker) | Sonnet 5.5, medium | **in progress** |
| S-141 | **Build: tune the scatter from the playtest** (`research/scatter.md`). Read Mikey's notes, the overlay line `scatter:` in the marks, and the frame time with the scatter on and off (dev menu). Change the defaults on `BfScatter` (density, size scale, ranges, collision) and `ScatterParams::default()` with them. If the draw calls or the chunk build time are too high on the Intel GPU, use the ideas in `inbox.md`. Also find the hitches of the Benchmark on seed 1 with the scatter (worst 45.8 ms, `research/gpu-speed.md`). | Claude Code | Sonnet 5.5, medium | waiting on Mikey's play (S-137) |
| S-142 | **Brain (planner): plan demo 2, caves and water** (R3, R4, R24, R25). Write the research as articles in the wiki (R26). Write `plan/demo-2.md` (the spec) and change `plan/tech.md`. Add the build rows, so that Mikey sees water and caves early and the game stays at 60 fps on Intel graphics (R5). | Lead chat (cloud worker) | Opus 5.5, high | **in progress** |
| S-143 | **Play: the integrated-graphics test on the Vivobook**, the same as S-137, when Mikey has the Vivobook again. | Mikey's Vivobook | Mikey | waiting on the Vivobook |

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
