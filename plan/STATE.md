# State

Updated: 2026-11-14 (S-111 done: Mikey's notes on climbing, coming apart, gamepad and the zone edge)

Write this file again each time. Do not add to the end. Keep it under
90 lines.

## Phase

**Demo 1** (`demo-1.md`). In this demo, the player walks, jumps and
climbs through one biome under smog. The skeleton is built. Rust makes
a zone of scorched highlands, and Terrain3D shows it (see
`game/README.md`). The teddy bear walks and jumps on it, with a
third-person camera. Smog, climbing and scatter are not built yet.

## Agenda

To start a session, Mikey opens the place in the "Where" column. He
selects the model and effort, then types "Run S-NNN". At the end of a
session, the session deletes its row. It changes the rows that it
unblocks to **ready**. It adds follow-up rows at the bottom.
Next number: S-116.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-113 | **Build: gamepad and dev menu** (R13). The Back/Select button opens a dev menu. The d-pad and A use it: fly camera, overlay, new zone, seed. All game actions work on the gamepad. | Claude Code | Sonnet 5.5, medium | **ready** |
| S-107 | **Build: climbing** (R11, `demo-1.md`). Automatic start when the bear pushes into a steep surface. Move in all directions, jump off, automatic stop on flat ground, climb up onto ledges, stamina. A grip value for each surface type: slow slide, full slide, momentum. Keep it cheap: a few ray casts for each frame, no physics bodies while the bear climbs. | Claude Code | Opus 5.5, high (or Fable 5.2, medium) | **ready** |
| S-105 | **Build: smog and heat.** Distance fog and height fog, the sun, the sky, dust particles, heat shimmer, colour grading, render scale 0.75. Only Mobile renderer features. If Mikey says yes to the PS1-like look, add it here: a low internal resolution, nearest-pixel upscale, dithering and vertex snapping. | Claude Code | Sonnet 5.5, high | waiting on Mikey's answer about the PS1 look |
| S-114 | **Build: tumble and coming apart** (R11, R12). When the bear tumbles, it becomes a simple ragdoll (about 10 Jolt bodies, for the bear only). A hard hit can remove a limb. The limb stays in the world. The bear moves and climbs with the limbs it has: without legs, it crawls. | Claude Code | Opus 5.5, high | waiting on S-107 |
| S-115 | **Build: the smoke edge** (R14, `demo-1.md`). The seed and the time set the shape of the smoke wall around the angle of the zone. In the smoke, the fog becomes thick and a timer starts. If the bear stays too long or goes too far, it dies. Use cheap smoke: textured cards and a moving mesh, not volumetric fog. | Claude Code | Sonnet 5.5, high | waiting on S-105 |
| S-106 | **Play: the speed test.** Download the newest zip. Run the game at 1080p on both laptops. Walk across the zone with F3 on. Tell Claude the fps and the draw calls at some places. | Mikey's laptops | Mikey | waiting on S-105 |
| S-108 | **Build: ground scatter.** Make rocks, pebbles and dead shrubs in Rust. Draw them with MultiMesh and visibility ranges. Stay inside the numbers from S-106. For the shrubs, look at TanTan's `shrubbery` crate (space colonization, deterministic, MIT/Apache) [unverified: whether it works without Bevy]. | Claude Code | Sonnet 5.5, medium | waiting on S-106 |
| S-109 | **Brain** with Mikey: play the full demo, decide what to fix, and plan demo 2 (caves). | Project chat | Opus 5.5, medium | waiting on S-114, S-115 and S-108 |

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
