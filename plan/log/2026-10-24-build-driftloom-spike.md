---
date: 2026-10-24
session: build
model: Claude Opus 5.5 (claude.ai)
---
# Build: Driftloom's core in the game (spike 2)

## Changed

- `rust/driftloom-godot/`: the GDExtension; `rust/build.sh`, README.
- `game/addons/driftloom/`, `game/autoload/music.gd`,
  `game/tests/music_test.gd`; web export uses Extensions Support.
- CI builds the extension for the browser before exporting.
- `prototypes/02-driftloom-spike.md`; V-008 answered.
- Driftloom repo: PR #258 names Broken Friends the one proof game
  (docs only; waiting for Mikey to merge).

## Decided by Mikey

- Broken Friends is the one and only game that proves Driftloom's core;
  no mods for the other three games, maybe another time.

## Still open

- Hearing it on a phone. Android and Windows libraries.

## Next

- Mikey listens; Driftloom's phase 2 brings new music in Rust.
