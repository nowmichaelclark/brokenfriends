# State

Updated: 2026-11-14 (S-107 done: the bear climbs, with stamina and grip)

Write this file again each time. Do not add to the end. Keep it under
90 lines.

## Phase

**Demo 1** (`demo-1.md`). In this demo, the player walks, jumps and
climbs through one biome under smog. The skeleton is built. Rust makes
a zone of scorched highlands, and Terrain3D shows it (see
`game/README.md`). The teddy bear walks and jumps on it, with a
third-person camera. The bear climbs steep surfaces, with stamina and
a grip value for each ground type. Smog and scatter are not built yet.

## Agenda

To start a session, Mikey opens the place in the "Where" column. He
selects the model and effort, then types "Run S-NNN". At the end of a
session, the session deletes its row. It changes the rows that it
unblocks to **ready**. It adds follow-up rows at the bottom.
Next number: S-118.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-113 | **Build: gamepad and dev menu** (R13). The Back/Select button opens a dev menu. The d-pad and A use it: fly camera, overlay, new zone, seed. All game actions work on the gamepad. | Claude Code | Sonnet 5.5, medium | **ready** |
| S-105 | **Build: smog and heat** (R7, R15). Distance fog and height fog, the sun, the sky, dust particles, heat shimmer and colour grading. Full 1080p, within the R15 detail budget. No PS1 effects. Only Mobile renderer features. Read `research/smog-and-budget.md` first. | Claude Code | Sonnet 5.5, high | **ready** |
| S-114 | **Build: tumble and coming apart** (R11, R12). When the bear tumbles, it becomes a simple ragdoll (about 10 Jolt bodies, for the bear only). A hard hit can remove a limb. The limb stays in the world. The bear moves and climbs with the limbs it has: without legs, it crawls. Read `research/ragdoll-and-limbs.md` first. The climb gives the fall velocity in `BfBear::let_go`. | Claude Code | Opus 5.5, high | **ready** |
| S-115 | **Build: the smoke edge** (R14, `demo-1.md`). The seed and the time set the shape of the smoke wall around the angle of the zone. In the smoke, the fog becomes thick and a timer starts. If the bear stays too long or goes too far, it dies and starts again at the start point (R16). Use cheap smoke: textured cards and a moving mesh, not volumetric fog. Read `research/smog-and-budget.md` first. | Claude Code | Sonnet 5.5, high | waiting on S-105 |
| S-106 | **Play: the speed test.** Download the newest zip. Run the game at 1080p on both laptops. Walk across the zone with F3 on. Tell Claude the fps, the draw calls at some places, and the time to make a new zone (F4). If you have a well-made PS1 PC port, also note its fps on the same laptop (R15). | Mikey's laptops | Mikey | waiting on S-105 |
| S-108 | **Build: ground scatter.** Make rocks, pebbles and dead shrubs in Rust. Draw them with MultiMesh and visibility ranges. Stay inside the numbers from S-106. For the shrubs, look at TanTan's `shrubbery` crate (space colonization, deterministic, MIT/Apache) [unverified: whether it works without Bevy]. | Claude Code | Sonnet 5.5, medium | waiting on S-106 |
| S-109 | **Brain** with Mikey: play the full demo, decide what to fix, and plan demo 2 (caves). | Project chat | Opus 5.5, medium | waiting on S-114, S-115 and S-108 |
| S-116 | **Play: the climb test.** Download the newest zip. Climb with the gamepad for ten minutes, with F3 on. Tell Claude what you feel: the climb speed, how long the stamina lasts, the slide on ash rock and sand, the jump off, the move onto a ledge, and each place where the bear gets stuck or shakes. | Mikey's laptops | Mikey | **ready** |
| S-117 | **Build: climb fixes from the play notes.** Change the climb defaults and fix the problems that S-116 finds. Done when each note of S-116 has a change or a reason. | Claude Code | Sonnet 5.5, medium | waiting on S-116 |

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
