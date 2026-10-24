# Driftloom inside Broken Friends

`driftloom-godot/` is a Godot extension, written in Rust with
[godot-rust](https://godot-rust.github.io), that links Driftloom's audio
core (`github.com/nowmichaelclark/driftloom`, `core/`, pinned to one
commit in `Cargo.toml`). It gives Godot one class, `DriftloomPlayer`:

| Method | What it does |
|---|---|
| `load_notes(text)` | Load a Driftloom note list (`.dlnotes`); returns "" or an error |
| `render_stereo(frames)` | The next frames, for `AudioStreamGeneratorPlayback.push_buffer` |
| `is_finished()`, `sample_rate()`, `length_seconds()` | About the loaded list |
| `golden_line(text)` | Render a whole list and return Driftloom's hash line, to check builds |
| `looping`, `gain` | Properties |

The game's `Music` autoload (`game/autoload/music.gd`) uses it, and is
silent when the library isn't built.

Today Driftloom's core is the synthesizer; the part that writes the
notes is still JavaScript in the app until Driftloom's phase 2. So the
game plays note lists: six of Driftloom's golden loops, in
`game/addons/driftloom/loops/`.

## Building

```bash
rust/build.sh linux     # for the laptop: needs Rust (rustup)
rust/build.sh web       # for the browser: see below
```

Each copies the library into `game/addons/driftloom/bin/`, which is not
committed. CI builds the web library on every push
(`.github/workflows/web.yml`).

### The browser build

Godot's standard web engine can't load extensions; its `web_dlink`
engine can (the Web export preset has **Extensions Support** on, single
threaded). The extension must be built to match it:

- **Emscripten 4.0.20**, the version Godot 4.7.1's web engine was built
  with (it prints this in the browser console). Install with emsdk:
  `./emsdk install 4.0.20 && ./emsdk activate 4.0.20 && source ./emsdk_env.sh`.
- **Rust nightly-2026-10-24** with `rust-src` and the
  `wasm32-unknown-emscripten` target (std is rebuilt with `-Zbuild-std`).
- **`GODOT4_BIN`** set to a Godot 4.7.1 binary: the web build generates
  its Godot API from it (godot-rust's `api-custom`).
- **Panics abort** instead of unwinding. Rust's Emscripten target now
  unwinds with WebAssembly exceptions, which Godot's web engine doesn't
  have; an unwinding build fails to load with "`__cpp_exception`: tag
  import requires a WebAssembly.Tag". `build.sh` rebuilds std with
  `panic_abort`.
- Flags for the side module are in `driftloom-godot/.cargo/config.toml`.
  (The godot-rust book's `-Zemscripten-wasm-eh=false` no longer exists
  in nightly.)

## Checking

```bash
godot --path game -- --music-test     # renders the golden loops in the game
```

On the web, open the game with `#music-test` at the end of the address
and read the browser console. Both print `MUSIC OK` when every hash
matches Driftloom's own (`game/addons/driftloom/loops/golden.hashes`).

Measured 2026-10-24 on the build machine: all six loops bit-identical on
Linux and in Chromium; Linux about 22-32x real time, the browser about
19x; no stream underruns in either while the game runs. Not yet measured
on a phone.

Not built yet: Android (needs the Android NDK) and Windows libraries.
