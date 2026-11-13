# State

Updated: 2026-11-13 (the 3D version starts: PC, Godot and Rust)

Write this file again each time. Do not add to the end. Keep it under
90 lines.

## Phase

**Demo 1** (`demo-1.md`). In this demo, the player walks, jumps and
climbs through one biome under smog. Nothing is built yet.

## Agenda

To start a session, Mikey opens the place in the "Where" column. He
selects the model and effort, then types "Run S-NNN". At the end of a
session, the session deletes its row. It changes the rows that it
unblocks to **ready**. It adds follow-up rows at the bottom.
Next number: S-111.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-101 | **Build: skeleton.** Make the Cargo workspace (`bf-worldgen` and `bf-game`) with the dev profiles in `tech.md`. Make a `.gdextension` file with `reloadable = true`. Make a Godot 4.7.1 project that uses the Mobile renderer. Install a pinned version of Terrain3D. Test: Rust makes a noise height map, and Terrain3D shows it. Add the F3 overlay, a `bacon` config and CI (`fmt`, `clippy`, `test`). Write `game/README.md`: how to build and run on Fedora and on Windows. **Done when:** the noise terrain shows, a Rust change reloads in the editor, and the rebuild time on Mikey's laptop is written down. | Claude Code | Opus 5.5, high | **ready** |
| S-102 | **Build: the teddy bear.** Make a Rust `CharacterBody3D`, 0.40 m tall, from simple shapes. Add walking and jumping (with coyote time and a jump buffer). Add a third-person camera that does not go into the terrain. Make all tuning values `#[export]`. Test on the terrain from S-101. | Claude Code | Sonnet 5.5, high | waiting on S-101 |
| S-104 | **Build: the scorched-highlands generator** in `bf-worldgen`. Use ridged noise, domain warp, the erosion filter, dry riverbeds and a seed. A preview tool writes PNG images. A dev key makes a new zone in the game. **Done when:** three seeds all look like real mountain ranges in the preview. | Claude Code | Opus 5.5, high | waiting on S-101 (can run at the same time as S-102) |
| S-103 | **Play:** Mikey walks and jumps. He changes the tuning values in the Inspector. He writes the problems in `play-notes.md`. | Mikey's laptop | Mikey (optional: Sonnet 5.5, low, for help with the notes) | waiting on S-102 |
| S-105 | **Build: smog and heat.** Add distance fog and height fog, the sun, the sky, dust particles, heat shimmer and colour grading. Set the render scale to 0.75. Use only features of the Mobile renderer. | Claude Code | Sonnet 5.5, medium | waiting on S-104 |
| S-106 | **Play: the speed test.** Run the game at 1080p on both laptops. Walk across the zone with F3 on. Write the fps and draw calls in `play-notes.md`. | Mikey's laptops | Mikey | waiting on S-105 |
| S-107 | **Build: climbing.** Find the wall, hold on to it, move along the surface, use stamina, and climb up onto ledges. Use the notes from S-103. | Claude Code | Opus 5.5, high | waiting on S-103 and S-105 |
| S-108 | **Build: ground scatter.** Make rocks, pebbles and dead shrubs in Rust. Draw them with MultiMesh and visibility ranges. Stay inside the numbers from S-106. | Claude Code | Sonnet 5.5, medium | waiting on S-106 |
| S-109 | **Brain** with Mikey: play the full demo, decide what to fix, and plan demo 2 (caves). | Project chat | Opus 5.5, medium | waiting on S-107 and S-108 |
| S-110 | **Combined run with Fable:** S-101, then S-104, then S-102, in one session. See "Combined runs" below. | Claude Code | Fable 5.2, medium | **ready** (use this row or S-101, not both) |

## Combined runs

A combined run does more than one row in one session. Use it when a
strong model is available for a short time.

1. Do the rows in the order that the row gives.
2. Finish each row fully before you start the next row. This includes
   the tests, `STATE.md`, the commit and the push. Each push is a save
   point. If the session stops, the next session starts at the first
   row that is still in the agenda.
3. Keep the context small. Do not read `archive/`. To read external
   docs (for example, the Terrain3D API or the erosion filter source),
   use a subagent. Tell it to send back only a short summary.
4. If the context becomes full, finish the current row, push, and
   stop. Do not start a new row with a full context.
5. At the end, delete the S-110 row.

S-110 does S-104 before S-102 for this reason: S-104 is the most
difficult row and gets the most from Fable. If the Fable time ends,
Sonnet can do S-102 easily.

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
