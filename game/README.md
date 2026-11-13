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
- **The scorched-highlands generator** (`bf-worldgen`, `highlands.rs`)
  makes a zone in these steps:
  1. Domain warp bends the positions, so the ranges are not straight.
  2. Ridged noise makes the mountain ranges and the valley floors.
  3. The erosion filter cuts gullies into the slopes.
  4. Fine noise makes the ground a little rough.
  5. The drainage step finds where rain goes. Deep closed basins get a
     flat dry lake bed. Paths that collect much rain become dry
     riverbeds. A river stops at the shore of a dry lake.
- **`BfTerrain`** makes a zone of 2048 × 2048 m from a seed on a worker
  thread. It gives the heights and the ground colours to the Terrain3D
  addon. Terrain3D draws the ground and makes the collision. Push F4
  to make a new zone with the next seed. The node sends the signal
  `zone_changed` when the zone is on the screen.
- **`BfDebugOverlay`** shows fps, frame time, draw calls, primitives,
  video memory and the seed. Push F3 to show or hide it.
- **`BfFlyCamera`** is a debug camera. Hold the right mouse button to
  look. Move with W, A, S, D, Q and E. Hold Shift to go faster.
- **`main.tscn`** is the start scene: terrain, sun, sky, camera and
  overlay.

The ground colours are: pale ochre on flat ground, rust and ash bands
on rock (steeper than about 35 to 50 degrees), and pale sand in
riverbeds and lake beds. One plain texture and a colour map make them.

## Preview images without Godot

Run this in `rust/`:

```
cargo run --release -p bf-worldgen --example preview -- --seed 1 --seed 7 --seed 42
```

For each seed, the tool writes four PNG images into `rust/preview/`: a
height image, a map with shadows, a view from the air, and a view from
the eyes of the bear. It also prints the time, the height range and
the parts of the zone that the bear can walk (less than 35 degrees) or
must climb (more than 50 degrees). In the cloud container, one zone
takes 2.3 s (release) or 2.8 s (dev build in the game).

## Change the generator

Select the `Terrain` node. The Inspector shows the generator values
(relief, ridge wavelength, erosion strength and more). Change a value,
then push F4 in the game, or call `regenerate()`. More values are in
`HighlandsParams` in `highlands.rs`. Those need a new compile.

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
- **Vertex spacing is 1 m.** The zone has 2048 × 2048 heights. Shapes
  smaller than 2 m do not show.
- **Physics engine: Jolt.** It is the default for new 3D projects in
  Godot 4.7 [unverified].
- **The window starts maximized** at a base size of 1920 × 1080.
- **Debug keys are fixed in code** (F3, F4, fly camera keys). They are
  not in the Input Map.
- **One file has the MPL-2.0 licence.** `erosion_filter.rs` is a port
  of the erosion filter by Rune Skovbo Johansen, as `plan/tech.md`
  says. His code has the MPL-2.0 licence, thus the port keeps it. All
  other files stay MIT. See `THIRD-PARTY-NOTICES.md`.
- **Dry lakes.** A closed basin deeper than 6 m keeps its shape. The
  lowest 20% of its depth is a flat dry lake bed.
- **Steep zones.** With peaks 300 to 600 m high in a zone of 2 km,
  the bear can walk on about 20 to 35% of the ground and must climb
  on about 35 to 50%. Lower `relief_m` or `valley_at` for more flat
  ground.
- **The zone border is open.** The terrain stops at the border.
- **Same world on all computers.** The generator uses its own `sin`,
  `cos` and `exp` (`detmath.rs`), not the functions of the operating
  system. A test on Windows did not run yet [unverified].
- **Light.** The sun and the sky in `main.tscn` are placeholders.
  S-105 makes the real light and smog.
