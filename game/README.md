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
- **`BfBear`** is the teddy bear, a `CharacterBody3D`, 0.40 m tall.
  Code makes its body from balls and capsules. It walks, jogs and
  jumps. The jump has coyote time and a jump buffer. When a new zone
  shows, the bear goes to the flattest place near the centre. If the
  bear is below the ground, the code puts it back on the ground.
- **`BfFollowCamera`** is the third-person camera. The mouse or the
  right stick turns it around the bear. It reads the heightmap and
  comes nearer when the ground is in the way.
- **`BfDebugOverlay`** shows fps, frame time, draw calls, primitives,
  video memory and the seed. Push F3 to show or hide it.
- **`BfFlyCamera`** is a debug camera. Push F2 to use it, and F2 again
  to go back. Hold the right mouse button to look. Move with W, A, S,
  D, Q and E. Hold Shift to go faster. The bear does not move while
  this camera is active.
- **`BfAtmosphere`** makes the air. In code, it makes the sky, the
  sun, the smog and the colour grading. Depth fog and low height fog
  use one yellow-brown colour. The sky has a big pale sun disc. About
  300 dust particles move past the camera. A screen shader gives a
  small heat shimmer near the horizon. The node reads its values
  twice each second, so a change in the Remote Inspector shows soon.
- **`main.tscn`** is the start scene: atmosphere, terrain, bear,
  cameras and overlay.

## Play the latest build (Mikey)

GitHub builds the game after each push to `main`. You do not need
Rust or the Godot editor.

1. Open https://github.com/nowmichaelclark/brokenfriends/releases/tag/game-latest
2. Download `broken-friends-linux.zip` (about 30 MB). On Windows,
   download `broken-friends-windows.zip`.
3. Unzip it. On Fedora:
   `unzip -o ~/Downloads/broken-friends-linux.zip -d ~/Games`
4. Start the game. On Fedora:
   `~/Games/broken-friends-linux/BrokenFriends.x86_64`. On Windows,
   double-click `BrokenFriends.exe`.

`BUILD.txt` in the folder gives the commit of the build. To get a
newer build, do the steps again. Step 3 replaces the old files.

If the game does not start, run it from a terminal and send the text
that it prints to Claude. On Windows, run
`BrokenFriends.console.exe` to see the text.

## Controls

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move | W, A, S, D or the arrow keys | left stick |
| Jog | hold Shift | hold the right shoulder button |
| Jump | Space | bottom button (A) |
| Turn the camera | mouse | right stick |
| Release the mouse | Esc (click the window to hold it again) | |
| Fly camera | F2 | |
| Debug overlay | F3 | |
| New zone | F4 | |

All tuning values of the bear and the camera are in the Inspector
(select `Bear` or `FollowCamera`). The movement maths is in
`rust/game/src/motion.rs`, with unit tests.

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

## Build and run on Fedora (for development)

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

## Build and run on Windows (for development)

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
Mikey does not build on his laptop (decision R10).

## Tests

- In `rust/`: `cargo fmt`, `cargo clippy --workspace --all-targets`
  and `cargo test`.
- Load test without a window:
  `godot --headless --path game --quit-after 300`. On a new clone, run
  `godot --headless --path game --import` one time before this.
- CI (`.github/workflows/ci.yml`) runs `fmt`, `clippy` and `test` on
  each push. CI does not start Godot.
- The game build (`.github/workflows/game-build.yml`) builds the Rust
  library for Linux and Windows. Then Godot exports the game with
  `export_presets.cfg`. A smoke test starts the Linux build headless
  and checks that it makes a zone. The zips go on the `game-latest`
  release.
- To export by hand: build with `cargo build --release`, copy
  `libbf_game.so` from `rust/target/release/` to `rust/target/debug/`,
  run `godot --headless --path game --import`, then
  `godot --headless --path game --export-release "Linux" ../build/linux/BrokenFriends.x86_64`.
  The import run can crash when it quits; the import is still
  complete.

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
- **All keys are fixed in code.** They are not in the Input Map, thus
  the player cannot change them.
- **Jog.** The bear walks at 1.4 m/s. It jogs at 3.2 m/s while Shift
  is down. The jump is 0.55 m high.
- **Camera.** Third person, 1.8 m behind the bear. The camera avoids
  the terrain only. It does not avoid other objects yet.
- **Start point.** The flattest place within 200 m of the centre. This
  can be a hilltop.
- **The bear has no animation.** The body turns to the move direction.
  The arms and legs do not move.
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
- **Smog.** The first values (fog density 0.006, low smog under 90 m,
  saturation 0.8) are guesses. Nobody saw them in the Mobile
  renderer yet [unverified]. Mikey tells Claude what he sees.
- **One smog colour.** Godot 4 has one fog colour. The low smog has
  the same colour as the far smog and is only thicker.
- **Heat shimmer.** The shimmer is on. It copies the screen each
  frame, so it can cost time on integrated graphics. To test, turn off
  `heat_shimmer_enabled` on the `Atmosphere` node and compare the fps.
- **Frame cost.** The atmosphere has no measured cost. The parts that
  can cost time are the shimmer, the dust (300 particles, small) and
  the shadows (2 cascades, 100 m).
