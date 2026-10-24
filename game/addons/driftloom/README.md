# Driftloom in Broken Friends

Driftloom's audio core (`github.com/nowmichaelclark/driftloom`, `core/`)
as a Godot extension, built from `rust/driftloom-godot/` at the repo
root. Decision 0016: Broken Friends is the one game that proves the
core.

- `driftloom.gdextension`: where Godot finds the built library for each
  platform. The libraries in `bin/` are build output, not committed.
- `loops/`: six of Driftloom's golden note lists, and `golden.hashes`,
  the exact hashes every build must render them to.

The game runs without the library: `Music` (in `autoload/`) checks for
the `DriftloomPlayer` class and stays silent if it isn't there.

Build the library for the laptop with `rust/build.sh linux`; the web
build is made by `rust/build.sh web` and by CI (`.github/workflows/web.yml`).
See `rust/README.md`.
