# The game: what the build is now

This folder is the Godot 4.7.1 project. The game logic is in Rust, in
`../rust/`. This file tells what the build has, how to build it and
what it assumes.

## What the build has

- **A Rust workspace** (`../rust/`) with two crates:
  - `bf-worldgen` makes the world. It does not use Godot. It has seeded
    gradient noise and a heightmap type. A heightmap is a grid of
    ground heights.
  - `bf-game` is the GDExtension. A GDExtension is a library that
    Godot loads. `bf.gdextension` tells Godot where the library is.
- **`BfTerrain`** makes a zone of 2048 × 2048 m from a seed (test
  hills from noise). It gives the heights to the Terrain3D addon as a
  32-bit float image. Terrain3D draws the ground and makes the
  collision.
- **`BfDebugOverlay`** shows fps, frame time, draw calls, primitives,
  video memory and the seed. Push F3 to show or hide it.
- **`BfFlyCamera`** is a debug camera. Hold the right mouse button to
  look. Move with W, A, S, D, Q and E. Hold Shift to go faster.
- **`main.tscn`** is the start scene: terrain, sun, sky, camera and
  overlay.

The ground has no textures yet, thus Terrain3D draws a checker
pattern.

## Build and run on Fedora

1. Install Rust: `curl https://sh.rustup.rs -sSf | sh`. The workspace
   needs Rust 1.94 or later.
2. Download Godot 4.7.1 (standard version, not .NET) from
   godotengine.org. Put the program on your `PATH` with the name
   `godot`.
3. Install the pinned Terrain3D addon (v1.1.0):
   `tools/install-terrain3d.sh`. The script needs `curl` and `unzip`.
4. Build the library: `cd rust && cargo build`. The first build takes
   some minutes.
5. Open the editor: `godot --path game -e`. The first start imports
   the addon files.
6. Push F5 to start the game.

## Build and run on Windows

1. Install Rust from rustup.rs. Use the default MSVC toolchain. The
   installer also offers the Visual Studio Build Tools. Install them.
2. Download Godot 4.7.1 (standard version) from godotengine.org.
3. Install the pinned Terrain3D addon:
   `powershell -ExecutionPolicy Bypass -File tools\install-terrain3d.ps1`
4. Build the library: `cd rust`, then `cargo build`.
5. Start Godot. Import and open `game\project.godot`.
6. Push F5 to start the game.

## Fast changes

1. Install bacon one time: `cargo install --locked bacon`.
2. In `rust/`, run `bacon`. It builds the library after each save.
3. Click the Godot editor. The editor loads the new library.
4. Push F5.

Each tuning value is an `#[export]` field. Change it in the Inspector.
While the game runs, select the node in the Remote scene tree. If you
change the fields or the base class of a Rust class, restart the
editor.

**Rebuild times.** In the cloud container (4 cores), the first build
takes 1 min 42 s. A build after a change in `bf-game` takes about 2 s.
These numbers are not valid for Mikey's laptop.

| Computer | First build | Build after a small change |
|---|---|---|
| Mikey's laptop | (Mikey: write the time here) | (Mikey: write the time here) |

## Tests

- In `rust/`: `cargo fmt`, `cargo clippy --workspace --all-targets`
  and `cargo test`.
- Load test without a window:
  `godot --headless --path game --quit-after 300`. On a new clone, run
  `godot --headless --path game --import` one time before this.
- CI (`.github/workflows/ci.yml`) runs `fmt`, `clippy` and `test` on
  each push. CI does not start Godot.

## Assumed (Mikey can change these)

- **The addon is not in git.** The install scripts download Terrain3D
  v1.1.0 and check its SHA-256 checksum. This keeps 47 MB of binary
  files out of the repo. The cost: one more step after a clone.
- **The addon folder is `addons/terrain_3d/`.** Terrain3D needs this
  name.
- **Vertex spacing is 1 m.** The zone has 2048 × 2048 heights.
- **Physics engine: Jolt.** It is the default for new 3D projects in
  Godot 4.7 [unverified].
- **The window starts maximized** at a base size of 1920 × 1080.
- **Debug keys are fixed in code** (F3, fly camera keys). They are not
  in the Input Map.
