# Build session: make one agenda row playable

Use Claude Code only. This session builds, runs and pushes.

## Read first

Read these files: `CLAUDE.md`, your row in `STATE.md`, `tech.md`,
`demo-1.md`, and `game/README.md` (after S-101 makes it). Then read
only the code that you will change.

## Steps

1. Write the "done when" of your row again in your own words. If the
   row has no "done when," add one to the row first.
2. Look for references before you code. Read the files in
   `plan/research/` that are about your topic. Then use a subagent to
   find 2 or 3 open-source repos or articles that solve the same
   problem. Tell it to send back a short summary: the method, the
   cost, and the licence. Write the useful points in
   `plan/research/<topic>.md`. Do not copy code with a licence that
   does not fit MIT.
3. Build in small steps. After each step, compile and run the game
   headless. If the change is visible, make a screenshot (see
   `tech.md`, "Tests in Claude Code").
4. Make each new tuning value an `#[export]` field. Give it a good
   default value and a one-line doc comment. It then shows in the
   Inspector.
5. Put pure logic (generation, maths, state machines) in
   `bf-worldgen` or in plain Rust modules with unit tests. Keep the
   code that talks to Godot thin.
6. If the plans do not answer a design question, build the simplest
   version. Add it to "Assumed" in `game/README.md`. Ask Mikey in your
   summary.
7. If a change can cost frame time, write it down. Also write how to
   measure it on Mikey's laptop.

## Done when

- The "done when" of the row is true. A test, a screenshot or a
  printed number shows it.
- `cargo fmt`, `clippy` and `test` show no problems. The headless run
  loads without errors.
- `game/README.md` tells what the build is now and what it assumes.
  It does not tell the history of the steps.
- `STATE.md` is updated (see `CLAUDE.md`, "End of every agenda row").

## Do not

- Do not plan past your row. Write ideas in `inbox.md`.
- Do not commit screenshots, builds or `target/`.
- Do not add game logic in GDScript (see `CLAUDE.md`, "Rust first").
