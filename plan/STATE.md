# State

Updated: 2026-11-14 (S-115: the smoke edge)

Write this file again each time. Do not add to the end. Keep it under
90 lines.

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
writes one zip for each play session. Scatter is not built yet.

## Agenda

To start a session, Mikey opens the place in the "Where" column. He
selects the model and effort, then types "Run S-NNN". At the end of a
session, the session deletes its row. It changes the rows that it
unblocks to **ready**. It adds follow-up rows at the bottom.
Next number: S-132.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-113 | **Build: gamepad dev menu and tests** (R13, R22, `diagnostics.md`). Hold Back/Select to open a dev menu (a tap of Back is the playtest mark). The d-pad and A use it: fly camera, overlay, new zone, seed, "Save playtest." A "Tests" page with the benchmark and the draw distance test. All game actions work on the gamepad. "Save playtest" calls `save_playtest()` on the `bf_recorder` group; a hold of Back is free (a tap is a mark). A "Limbs" page calls `set_limb()` and `restore_limbs()` on the bear (the keys 1 to 4 and 0 of S-122). | Claude Code | Sonnet 5.5, high | **ready** |
| S-114 | **Build: tumble and coming apart** (R11, R12). When the bear tumbles, it becomes a simple ragdoll (about 10 Jolt bodies, for the bear only). A hard hit can remove a limb. The limb stays in the world. The bear moves and climbs with the limbs it has: without legs, it crawls. Read `research/ragdoll-and-limbs.md`, `research/bear-animation.md` (topics 4 and 6, build order steps 11 to 13 and 16) and `bear.md` first. Use the parts and seams of `BfBearModel` (`bear_shape.rs`). The climb gives the fall velocity in `BfBear::let_go`. Lose limbs with `BfBear::change_limbs`: the gait table (`limbs.rs`) and the animation follow. | Claude Code | Opus 5.5, high | **ready** |
| S-118 | **Build: tie the draw distance to the smog.** The camera draws up to 3000 m now, behind the smog, so the smog saves no time yet. Set the camera far plane, the Terrain3D draw range and the visibility ranges to match the smog distance. Measure the effect with the draw distance test (`diagnostics.md`) in the cloud, and leave the real numbers to Mikey's next playtest. The look of the smog stays as it is (Mikey likes it). | Claude Code | Sonnet 5.5, high | waiting on S-113 |
| S-124 | **Build: playtest replay** (`diagnostics.md`, "Later: replay"). Record the input of each physics tick in the playtest. A headless command replays a playtest with its seed and compares the bear's path with the recorded positions. | Claude Code | Opus 5.5, high | **ready** |
| S-125 | **Play: the next playtest.** Download the newest zip (it has S-117, S-121, the walk animation of S-120, the climbing hands and gaits of S-122 and the smoke edge of S-115) and play for about 15 minutes. Walk slowly, walk, jog, jump, and go across slopes and sand. Tap Back when something is wrong. Send the zip from `playtests/`. Notes about feel and taste are optional. For the animation: is the walk too busy? Is the bear soft enough, or too wobbly? Also climb a wall and look at the hands. On a flat place, push 1 to 4 to remove limbs (0 gives them back): hop, crawl, and try a climb with one arm. For the smoke: walk to the wall at the edge of the zone, go in for a few seconds and come out, then stay in until the bear dies. Can you see the wall from far away? Is the time in the smoke (12 s) too short or too long? Does the dark fog feel fair? | Mikey's laptops | Mikey | **ready** |
| S-108 | **Build: ground scatter.** Make rocks, pebbles and dead shrubs in Rust. Draw them with MultiMesh and visibility ranges. Stay inside the speed numbers from the S-125 playtest. For the shrubs, look at TanTan's `shrubbery` crate (space colonization, deterministic, MIT/Apache) [unverified: whether it works without Bevy]. | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-126 | **Build: tune the walk animation from the playtest.** Read Mikey's notes and `anim_us` in `samples.jsonl` of the playtest. Change the defaults on `BfBearModel` (stride, bounce, waddle, springs) and keep `AnimParams::default()` in `anim.rs` the same. Fix what looks wrong at the marks. | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-127 | **Build: obstacle framework and the first natural obstacles** (R23, `obstacles.md`). Obstacles as seeded data. Find places after the terrain is made. Build four obstacles: crumbling ash crust, rolling stones, gusts, and vents (push and launch). Each has a tell before it acts. Log each hit, launch and fall in the playtest recorder. | Claude Code | Opus 5.5, high | **ready** |
| S-128 | **Build: obstacles that help, and sinking sand** (`obstacles.md`). Stepping stones that appear and vanish in a rhythm, updrafts, and sinking sand. | Claude Code | Sonnet 5.5, high | waiting on S-127 |
| S-129 | **Build: limb-ripping obstacles** (`obstacles.md`). Thorn snags, rock teeth and falling rocks that pin a limb. They use the seam health and limb loss of S-114. | Claude Code | Opus 5.5, high | waiting on S-114 and S-127 |
| S-130 | **Build: tune the climbing hands and the gaits from the playtest** (R18). Read Mikey's notes and the `limb_off`, `gait` and `grab` events. Change the defaults of the holds (`hold_step_m`, `hand_reach`, `hang_*`) and of the gaits (hop, lie, crawl, the speed scales on `Bear`). Keep `AnimParams::default()` and `GaitTuning::default()` the same as the exported defaults. | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-131 | **Build: tune the smoke edge from the playtest** (R14, `demo-1.md`). Read Mikey's notes, `smoke_m` and `smoke_s` in `samples.jsonl`, and the events `smoke_enter`, `smoke_leave` and `death:smoke:*` with their shots. Change the defaults on `BfSmokeEdge` (limits, wall speed, opacity, haze, card count). Check the frame time near the wall (overdraw) and cut cards if it is high. | Claude Code | Sonnet 5.5, medium | waiting on S-125 |
| S-109 | **Brain** with Mikey: play the full demo, decide what to fix, and plan demo 2 (caves). | Project chat | Opus 5.5, medium | waiting on S-114 and S-108 |

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

The cloud container cannot test the look with the Mobile renderer or
the speed. Mikey's play rows test them with the build from GitHub.

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
- **Fable 5.2:** Mikey has a limited amount. Use it for combined runs
  on the most difficult rows.
- Opus uses the limit of the $20 plan faster than Sonnet does. If the
  limit is low, use Sonnet 5.5, high, for an Opus row. Expect one more
  round of fixes.

## Blockers

None. The questions in `demo-1.md` are for Mikey. The build does not
wait for the answers.
