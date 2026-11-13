# State

Updated: 2026-11-14 (GitHub builds the game; Mikey plays, Claude tunes)

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
Next number: S-114.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-111 | **Play: the first start.** Download the Linux zip from the `game-latest` release and run it (see `game/README.md`, "Play the latest build"). Walk and jump. Use the fly camera (F2) to look at three zones (F4). Write what feels wrong and what looks wrong, in your own words. Answer the questions in `demo-1.md` and in `game/README.md` ("Assumed"). | Mikey's laptop | Mikey | **ready** |
| S-113 | **Build: tune from the play notes.** Mikey pastes his notes from S-111 at the start. Change the default values (movement, camera, generator) to match the notes. Fix small problems. Write the notes in `play-notes.md`. | Claude Code | Sonnet 5.5, medium | waiting on S-111 |
| S-105 | **Build: smog and heat.** Add distance fog and height fog, the sun, the sky, dust particles, heat shimmer and colour grading. Set the render scale to 0.75. Use only features of the Mobile renderer. | Claude Code | Sonnet 5.5, medium | **ready** |
| S-106 | **Play: the speed test.** Download the newest zip. Run the game at 1080p on both laptops. Walk across the zone with F3 on. Tell Claude the fps and the draw calls at some places. | Mikey's laptops | Mikey | waiting on S-105 |
| S-107 | **Build: climbing.** Find the wall, hold on to it, move along the surface, use stamina, and climb up onto ledges. Use the notes from S-111. | Claude Code | Opus 5.5, high | waiting on S-113 and S-105 |
| S-108 | **Build: ground scatter.** Make rocks, pebbles and dead shrubs in Rust. Draw them with MultiMesh and visibility ranges. Stay inside the numbers from S-106. | Claude Code | Sonnet 5.5, medium | waiting on S-106 |
| S-109 | **Brain** with Mikey: play the full demo, decide what to fix, and plan demo 2 (caves). | Project chat | Opus 5.5, medium | waiting on S-107 and S-108 |

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
