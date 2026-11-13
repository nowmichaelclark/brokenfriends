# State

Updated: 2026-11-13 (reboot: 3D, PC, Godot + Rust)

Rewrite this file, don't add to it. Keep it under 80 lines.

## Phase

**Demo 1** (`demo-1.md`): one biome to walk, jump and climb through,
under smog. Nothing is built yet.

## Agenda

Mikey starts a session by opening the place shown with that model and
effort and typing "Run S-NNN". A session ends by deleting its row,
flipping the rows it unblocks to **ready**, and adding follow-ups at
the bottom. Next number: S-110.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-101 | **Build: skeleton.** Set up the Cargo workspace (`bf-worldgen`, `bf-game`) with the dev profiles in `tech.md`, a reloadable `.gdextension`, and a Godot 4.7.1 project on the Mobile renderer. Pin and install Terrain3D. Spike: Rust makes a noise height map and Terrain3D shows it. Add the F3 overlay, a `bacon` config, and CI (`fmt`, `clippy`, `test`). Write `game/README.md` with how to build and run on Fedora and Windows. Done when the noise terrain shows up, a Rust edit reloads in the editor, and the rebuild time on Mikey's laptop is written down. | Claude Code | Opus 5.5, high | **ready** |
| S-102 | **Build: the teddy.** A Rust `CharacterBody3D` 0.40 m tall, built from simple shapes. Walk, jump (coyote time, jump buffer) and a third-person camera that doesn't clip. All tuning is `#[export]`. Test it on the S-101 terrain. | Claude Code | Sonnet 5.5, high | waiting on S-101 |
| S-104 | **Build: scorched highlands generator** in `bf-worldgen`: ridged noise, domain warp, the erosion filter, dry riverbeds and a seed. A preview tool writes PNGs, and a dev key regenerates in game. Done when three seeds all look like real mountain ranges in the preview. | Claude Code | Opus 5.5, high | waiting on S-101 (can run alongside S-102) |
| S-103 | **Play:** Mikey walks and jumps around and tunes the numbers in the Inspector. He notes what feels wrong in `play-notes.md`. | Mikey's laptop | Mikey (Sonnet 5.5, low if he wants help writing the notes) | waiting on S-102 |
| S-105 | **Build: smog and heat.** Distance and height fog, the sun, sky, dust particles, heat shimmer, colour grading and render scale 0.75, all within the Mobile renderer. | Claude Code | Sonnet 5.5, medium | waiting on S-104 |
| S-106 | **Play: the speed test.** 1080p on both laptops, a walk across the zone with F3 on. Note the FPS and draw calls in `play-notes.md`. | Mikey's laptops | Mikey | waiting on S-105 |
| S-107 | **Build: climbing.** Detect the wall, grab it, move along the surface, stamina, mantle onto ledges. Use S-103's notes. | Claude Code | Opus 5.5, high | waiting on S-103 and S-105 |
| S-108 | **Build: ground scatter.** Rocks, pebbles and dead shrubs made in Rust, drawn with MultiMesh and visibility ranges, staying within S-106's numbers. | Claude Code | Sonnet 5.5, medium | waiting on S-106 |
| S-109 | **Brain** with Mikey: play the whole demo, decide what to fix, and plan demo 2 (caves). | Project chat | Opus 5.5, medium | waiting on S-107 and S-108 |

## How models were picked

- **Opus 5.5, high:** sessions with many unknowns or fiddly maths.
  That means the first setup (every later session stands on it),
  terrain generation and climbing.
- **Sonnet 5.5:** well-trodden building, like a character controller,
  fog or instancing. High effort when it's the first real code in an
  area, medium when it's mostly settings.
- On the $20 plan, Opus uses up the limit faster. If you run out, any
  Opus row can drop to Sonnet 5.5 at high effort. Expect an extra fix
  round.

## Blockers

None. Mikey's answers to the questions in `demo-1.md` help, but the
build doesn't wait for them.
