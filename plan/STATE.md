# State

Updated: 2026-11-14 (S-133: the autoplay bot)

Write this file again each time. Do not add to the end (90 lines at most).

## Phase

**Demo 1** (`demo-1.md`): walk, jump and climb through one biome
under smog. Rust makes a zone of scorched highlands; Terrain3D shows it
(`game/README.md`). The bear walks, jumps and climbs (stamina, grip for
each ground type). Smog, sun, sky and dust are built (R19). Rust makes
the bear from SDF parts (R17) and poses them in each step (R18): planted
feet, springs, hands that hold the wall in turn, and a gait from the
limbs it has (keys 1 to 4 remove limbs). A wall of dark smoke surrounds
the zone (R14). It moves with the seed and the time, and it kills a
bear that stays in it (R16). Nobody has seen the smog, the smoke or the
animation in the Mobile renderer yet. The playtest recorder (R22)
writes one zip for each play session. A dev menu (hold Back, or F10) has
the dev tools, the limbs, "Save playtest" and a "Tests" page. Each zone
has about 200 seeded obstacles (R23): ash crusts, rolling stones, gusts
and vents, and (S-128) stepping stones, updrafts and sinking sand.
Nobody played them yet. The draw distance follows the smog (767 m).
An autoplay bot plays the bear through virtual gamepad input: each
game build runs a tour on 3 seeds, and a nightly run wanders on 8
seeds. Scatter is not built yet.

## Agenda

To start a session, Mikey opens the place in the "Where" column. He
selects the model and effort, then types "Run S-NNN". At the end of a
session, the session deletes its row. It changes the rows that it
unblocks to **ready**. It adds follow-up rows at the bottom.
Next number: S-135.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-114 | **Build: tumble and coming apart** (R11, R12). When the bear tumbles, it becomes a simple ragdoll (about 10 Jolt bodies, for the bear only). A hard hit can remove a limb. The limb stays in the world. The bear moves and climbs with the limbs it has: without legs, it crawls. Read `research/ragdoll-and-limbs.md`, `research/bear-animation.md` (topics 4 and 6, build order steps 11 to 13 and 16) and `bear.md` first. Use the parts and seams of `BfBearModel` (`bear_shape.rs`). The climb gives the fall velocity in `BfBear::let_go`. Lose limbs with `BfBear::change_limbs`: the gait table (`limbs.rs`) and the animation follow. | Claude Code | Opus 5.5, high | **ready** |
| S-124 | **Build: playtest replay** (`diagnostics.md`, "Later: replay"). Record the input of each physics tick in the playtest. A headless command replays a playtest with its seed and compares the bear's path with the recorded positions. | Claude Code | Opus 5.5, high | **ready** |
| S-125 | **Play: the next playtest.** Download the newest zip (it has S-117, S-121, the walk animation of S-120, the climbing hands and gaits of S-122 and the smoke edge of S-115) and play for about 15 minutes. Walk slowly, walk, jog, jump, and go across slopes and sand. Tap Back when something is wrong. Hold Back to open the dev menu: run the Benchmark and the Draw distance test from the Tests page (about 3 min each) and push "Save playtest". Send the zip from `playtests/`. Notes about feel and taste are optional. For the animation: is the walk too busy? Is the bear soft enough, or too wobbly? Also climb a wall and look at the hands. On a flat place, push 1 to 4 to remove limbs (0 gives them back): hop, crawl, and try a climb with one arm. For the smoke: walk to the wall at the edge of the zone, go in for a few seconds and come out, then stay in until the bear dies. Can you see the wall from far away? Is the time in the smoke (12 s) too short or too long? Does the dark fog feel fair? For the obstacles (S-127): F6 on the keyboard puts the bear at the next obstacle. Try each type (crust, stones, gust, push vent, launch vent). Which ones are fun, unfair or dull? Can you see the dust and the steam before they act? For the help obstacles (S-128): F6 goes on to stepping stones, updraft and sinking sand. Can you cross the stepping stones? Does the blink warn you in time? Jump into the updraft: is the float fun or too strong? Stand and walk on the sand: is it a danger, or only a slow patch? At the end, run "Autoplay tour" from the Tests page (about 9 min, the bot plays) and push "Save playtest" again. | Mikey's laptops | Mikey | **ready** |
| S-108 | **Build: ground scatter.** Make rocks, pebbles and dead shrubs in Rust. Draw them with MultiMesh and visibility ranges. Stay inside the speed numbers from the S-125 playtest. Put each prop in the group `bf_view_ranged` (S-118): it gets the visibility range from the smog. For the shrubs, look at TanTan's `shrubbery` crate (space colonization, deterministic, MIT/Apache) [unverified: whether it works without Bevy]. | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-126 | **Build: tune the walk animation from the playtest.** Read Mikey's notes and `anim_us` in `samples.jsonl` of the playtest. Change the defaults on `BfBearModel` (stride, bounce, waddle, springs) and keep `AnimParams::default()` in `anim.rs` the same. Fix what looks wrong at the marks. | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-129 | **Build: limb-ripping obstacles** (`obstacles.md`). Thorn snags, rock teeth and falling rocks that pin a limb. They use the seam health and limb loss of S-114. | Claude Code | Opus 5.5, high | waiting on S-114 |
| S-130 | **Build: tune the climbing hands and the gaits from the playtest** (R18). Read Mikey's notes and the `limb_off`, `gait` and `grab` events. Change the defaults of the holds (`hold_step_m`, `hand_reach`, `hang_*`) and of the gaits (hop, lie, crawl, the speed scales on `Bear`). Keep `AnimParams::default()` and `GaitTuning::default()` the same as the exported defaults. | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-131 | **Build: tune the smoke edge from the playtest** (R14, `demo-1.md`). Read Mikey's notes, `smoke_m` and `smoke_s` in `samples.jsonl`, and the events `smoke_enter`, `smoke_leave` and `death:smoke:*` with their shots. Change the defaults on `BfSmokeEdge` (limits, wall speed, opacity, haze, card count). Check the frame time near the wall (overdraw) and cut cards if it is high. | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-132 | **Build: tune the obstacles from the playtest.** Read Mikey's notes and "Obstacles" in `report.md` (hits, falls, escapes and launch results by type). Change the defaults on `Obstacles` and `Terrain`, and `SimParams::default()` with them. Include the stepping stones, updrafts and sinking sand of S-128. Check that the dust and steam show in the Mobile renderer (marks and `shots/`). | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-109 | **Brain** with Mikey: play the full demo, decide what to fix, and plan demo 2 (caves). | Project chat | Opus 5.5, medium | waiting on S-114 and S-108 |
| S-134 | **Build: fix what the first nightly autoplay finds** (`diagnostics.md`, "Autoplay"). Read `autoplay-nightly.md` on the `game-latest` release and the "Autoplay" notes of the game builds. Fix the bugs that it shows (falls through the ground, errors, many stuck or climb loop events at one place). If a task of the tour is often "missed" on a seed, make the bot better (`playtest/autoplay.rs`). Also look at the small put-back (0.26 m) on seed 7 at the stepping stones near (-256, 201): the collision and the heightmap do not agree there. | Claude Code | Sonnet 5.5, medium | waiting on the first nightly run (after the push of S-133) |

## Combined runs

A combined run does more than one row in one session. Use it when a
strong model is available for a short time. S-110 was the first one.

1. Do the rows in the order that the row gives.
2. Finish each row fully before you start the next row. This includes
   the tests, `STATE.md`, the commit and the push. Each push is a save
   point.
3. Keep the context small. Do not read `archive/`. To read external
   docs, use a subagent. Tell it to send back only a short summary.
4. If the context becomes full, finish the current row, push, and
   stop.
5. At the end, delete the row of the combined run.

The cloud container cannot test the Mobile look or the speed. Mikey's
play rows test them.

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
- **Fable 5.2:** limited. Use it for combined runs on the hardest rows.
- Opus uses the $20 plan limit faster. If the limit is low, use
  Sonnet 5.5, high, for an Opus row. Expect one more round of fixes.

## Blockers

None. The questions in `demo-1.md` are for Mikey. The build does not wait.
