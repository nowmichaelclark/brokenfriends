# For Claude Code sessions

This repo is **Broken Friends**, rebooted in November 2026 as a 3D PC
game: a teddy-bear-sized friend exploring a procedurally generated,
hot, smog-choked world. Built with Godot 4 and Rust, from scratch.

The old 2D game and its plans are in `archive/2d/` (also kept
whole on the `2d-concept` branch). Read them for ideas only. Don't build on their code or
follow their rules.

## Start of every session

1. Read `plan/STATE.md`. "Run S-NNN" means that agenda row.
2. Read the brief the row's type names (`plan/sessions/<type>.md`).
3. Read `plan/tech.md` (how the game is built) and the spec the row
   names (for now `plan/demo-1.md`).
4. Never edit `plan/MIKEY.md`. It is Mikey's.

## The rules that matter most

- **Rust first.** Game logic lives in Rust (`rust/`). This covers
  movement, camera, world generation, scatter and debug tools.
  Scenes (`.tscn`) and shaders (`.gdshader`) are fine. GDScript is
  only for a throwaway experiment, deleted in the same session.
- **Tuning numbers are `#[export]` fields**, so Mikey can change them
  in the Inspector (even while the game runs) without recompiling.
- **Scale.** 1 Godot unit = 1 metre, Y up. The teddy is 0.40 m tall.
  Everything else is real-world size around it.
- **Speed target.** 60 fps at 1080p on a laptop with integrated
  graphics, using the Mobile renderer and a 0.75 3D render scale. Only
  Mikey's laptops count as proof. A cloud container's numbers don't.
- **Seeded and deterministic.** The same seed always makes the same
  world. Never use an unseeded random source in generation.
- **Propose, don't decide.** If the plans don't settle a design
  question, build the simplest version, list it under "Assumed" in
  `game/README.md`, and ask Mikey. Decisions are Mikey's, in
  `plan/decisions.md`.
- **One job per session.** Do your agenda row. Put ideas for later in
  `plan/inbox.md`, one line each.
- **Tag guesses.** If a claim about an engine, crate or performance
  hasn't been checked, mark it `[unverified]`.

## Finishing a session

- Run `cargo fmt`, `cargo clippy` and `cargo test` in `rust/`, and fix
  what they report. Then open the game headless:
  `godot --headless --path game --quit-after 300`.
- Update `plan/STATE.md`. Delete your row, flip the rows it unblocks to
  **ready**, and add follow-ups (with the model and effort you'd
  recommend) at the bottom.
- Commit once: `<Session type>: <what changed>`. Finish on `main`:
  `git pull --rebase origin main`, then `git push origin HEAD:main`.
  If another session clashed with yours, keep both sides' rows and
  renumber yours.
- End your summary with **Weak spots**: at least three honest
  problems you see.
