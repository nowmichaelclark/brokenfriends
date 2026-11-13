# Build session: make one agenda row playable

Claude Code only. It builds, runs and pushes.

## Read first

`CLAUDE.md`, your row in `STATE.md`, `tech.md`, `demo-1.md`,
`game/README.md` (once it exists). Then only the code you'll change.

## How to run it

1. Restate your row's "done when". If it has none, write one into the
   row first.
2. Build in small steps. After each one, compile, run the game
   headless, and take a screenshot when the change is visible
   (`tech.md`, "Testing in Claude Code").
3. Put new tuning numbers in `#[export]` fields with sensible defaults
   and a one-line doc comment, so they show in the Inspector.
4. Pure logic (generation, maths, state machines) goes in
   `bf-worldgen` or plain Rust modules with unit tests. Godot-facing
   code stays thin.
5. If the plans don't settle a design question, build the simplest
   version, add it to "Assumed" in `game/README.md`, and ask Mikey in
   your summary.
6. Note anything that could cost frame time, and how you'd measure it
   on Mikey's laptop.

## Done when

- The row's "done when" holds, shown by a test, a screenshot or a
  printed number.
- `cargo fmt`, `clippy` and `test` are clean, and the headless run
  loads without errors.
- `game/README.md` says what the build is now and what it assumes. It
  isn't a history of the steps.
- `STATE.md` is updated (`CLAUDE.md`, "Finishing a session").

## Don'ts

- Don't plan past your row. Ideas go to `inbox.md`.
- Don't commit screenshots, builds or `target/`.
- Don't add GDScript game logic (`CLAUDE.md`, "Rust first").
