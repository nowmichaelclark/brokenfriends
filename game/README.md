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
  Its collision shape is a capsule. It walks, jogs and
  jumps. The jump has coyote time and a jump buffer. When a new zone
  shows, the bear goes to the flattest place near the centre. If the
  bear is below the ground, the code puts it back on the ground.
- **Climbing** (`rust/game/src/climb.rs`, R11). The bear climbs each
  surface that is too steep to walk (more than 50 degrees).
  - **Start.** Push into the surface. On the ground, the climb starts
    after 0.15 s. In the air, it starts at once.
  - **Move.** "Into the wall" is up. "Away from the wall" is down.
    Left and right move to the side.
  - **Stop.** The climb stops on ground flatter than 45 degrees. At
    the edge of a ledge, the bear moves onto the top in 0.2 s.
  - **Hop up (R20).** Jump makes a hop 0.35 m up the wall in 0.25 s.
    One hop uses 4% of the stamina (a normal climb of 0.35 m uses
    about 1.7%). A new hop starts only after the last hop ends.
  - **Jump off.** Jump with the stick away from the wall pushes the
    bear away from the wall. For 0.4 s after that, the bear cannot
    grab a wall.
  - **Let go (R21).** The let-go button drops the bear. It gets a
    small push away from the wall.
  - **Jog (R20).** A jog on the ground uses stamina: 20 s from full to
    empty. The stamina does not come back during a jog. When it is
    gone, the bear is tired and walks until 30% is back. The amount of
    stamina did not change.
  - **Stamina.** A ring near the bear shows it (`BfStaminaRing`). The
    stamina goes down only while the bear moves or slides. A steep
    wall uses more. At zero, the bear lets go, and the ring is red
    until 30% is back. The stamina comes back on the ground in 3 s.
  - **Grip.** Each ground type has a grip from 0 to 1. Rust rock
    holds fully (1.0). Ash rock is slippery (0.45): the bear slides
    down slowly. Riverbed sand does not hold (0.1): the slide gets
    faster, and the bear falls off with its speed. An object can set
    its own grip with the metadata `bf_grip`.
  - **Cost.** A climb step uses 5 or 6 ray casts. A walk step uses 2
    ray casts at most, and only while the player pushes a direction.
    No physics body moves while the bear climbs.
- **`BfBearModel`** is the visible bear (R17), a child of `BfBear`.
  Rust makes it when the game starts. No model file is used.
  - **Parts.** Each part is a union of SDF shapes (ellipsoids and
    round cones). An SDF gives the distance to the nearest surface.
    `fast-surface-nets` makes a mesh from it (`bear_shape.rs`).
  - **Separate meshes.** The body, the head, two ears, two arms and
    two legs are 8 meshes. The origin of each mesh is its joint (neck,
    shoulder, hip, ear base), so a part turns around its joint.
  - **Details.** The head has a long snout, button eyes, a stitched
    nose and a mouth of thread. The paws and the soles have pale felt
    pads. The body has a hump at the back. The arms reach past the
    hips.
  - **Seams.** The neck, the shoulders and the hips have a ring of
    stitches. The stitches have the colour of the fur and sit in it,
    so they do not look like dark lines. The ears have a seam position but no visible ring. S-114
    uses the seams for tearing.
  - **Fur.** One shader for all parts. Rust makes a small fur texture
    (short fibres, tufts, worn places). The shader also has a soft
    rim light and a little noise on the outline, so it looks fuzzy.
  - **Wet.** Each part has a wet value from 0 to 1. Wet fur is darker,
    flatter and a little shiny. Code sets it with
    `set_part_wet("LeftArm", 1.0)`. Nothing in the game makes the bear
    wet yet.
  - **Cost.** About 10 000 triangles and 8 draw calls (plus the
    shadow passes). The build takes about 9 ms at the start.
- **The animation** (R18, `rust/game/src/anim.rs`). The bear has no
  skeleton and no animation clips. In each physics step, Rust gives
  each part a new transform. The movement code stays in control; the
  animation only follows it.
  - **Stride wheel.** The distance that the bear moves turns a wheel.
    The wheel tells each leg when to stand and when to swing. A fast
    bear makes longer strides.
  - **Planted feet.** A foot that stands stays at one place in the
    world. One ray finds the ground for each step. A bear that stands
    moves a foot back below its hip if the foot is too far away.
  - **Aim IK.** A teddy bear has no knees or elbows. Each limb turns at
    its joint toward its target and stretches a little. IK (inverse
    kinematics) finds the turn of a limb from the place of its end.
  - **Body.** It bounces with the steps and rolls toward the foot that
    stands. It tilts with acceleration, leans with the speed and the
    slope, and sways a little when the bear stands.
  - **Ground.** On a slope, each foot is at the height of its ground,
    and the body comes down. On sand, the feet sink 1.5 cm and the
    steps are higher. On low grip (sand, ash rock), the planted feet
    slide and the steps become shorter and faster.
  - **Jump.** The body stretches at the start of a jump and squashes
    on the landing. In the air, the arms go up and the legs hang. In a
    long fall, the arms and the legs wave.
  - **Springs.** A spring type (`spring.rs`, `SecondOrder`) makes the
    soft parts. The paws, the head, the ears and the hump stay behind
    when the bear accelerates, and then swing back. The hump is a part
    of the body mesh; the fur shader moves it.
  - **On a wall,** the pose is the same as before: the arms reach up,
    and the limbs swing with the climb distance (S-122 changes this).
  - **Cost.** One step takes about 7 microseconds in the cloud
    container. It uses 2 ray casts at most while the bear moves, and
    none while the bear stands. The playtest recorder writes the time
    (`anim_us` in `samples.jsonl`).
- **`BfFollowCamera`** is the third-person camera. The mouse or the
  right stick turns it around the bear. It reads the heightmap and
  comes nearer when the ground is in the way.
- **`BfDebugOverlay`** shows fps, frame time, draw calls, primitives,
  video memory, the seed and the state of the bear (walk or climb,
  slope, grip, slide speed, stamina). Push F3 to show or hide it.
- **`BfFlyCamera`** is a debug camera. Push F2 to use it, and F2 again
  to go back. Hold the right mouse button to look. Move with W, A, S,
  D, Q and E. Hold Shift to go faster. The bear does not move while
  this camera is active.
- **`BfAtmosphere`** makes the air. In code, it makes the sky, the
  sun, the smog and the colour grading. Depth fog and low height fog
  use one yellow-brown colour. The sky has a big pale sun disc. About
  300 dust particles move past the camera. There is no heat shimmer
  (R19). The node reads its values
  twice each second, so a change in the Remote Inspector shows soon.
- **`BfRecorder`** is the playtest recorder (R22,
  `plan/diagnostics.md`). It is always on when the game has a window.
  - **Files.** It writes `session.json` (build, hardware, settings,
    zones), `samples.jsonl` (one line each second), `events.jsonl`,
    `frametimes.json` (all frame times in bins of 0.1 ms), `shots/`
    (JPEG, 960 x 540) and `marks/`.
  - **Each second:** fps, the average, P95 and max frame time, the CPU
    and GPU render time, draw calls, primitives, objects, memory,
    shader compiles, the bear (place, speed, state, slope, ground,
    grip, stamina, limbs), the camera and the input.
  - **Events:** hitch, low fps, stuck, climb loop, through the ground,
    the bear's state changes (jump, grab, slide, let go and why,
    mantle, long landing, restart), new zone, marks, and the errors
    and warnings of the Godot log.
  - **Screenshots:** one each 30 s, one at each mark, and one at a
    hitch or "stuck" (at most one each 10 s, 60 in total).
  - **Marks.** Tap Back on the gamepad or push F9. The recorder saves a
    screenshot and the frames of the last 10 s. "Mark 3 saved." shows
    on the screen.
  - **The end.** When the game quits, the recorder writes `report.md`
    and packs all files and the Godot log into one zip. If the game
    stopped without a normal quit, the next start packs that session.
    The folder keeps 200 MB of zips at most; the oldest go first.
  - **Where.** In `playtests/` next to the game. If that folder cannot
    be written, or in the editor, the files go to the Godot user
    folder (`~/.local/share/godot/app_userdata/Broken Friends/`).
  - **Cost.** About 0.07 ms in each frame in the cloud (the report
    shows the real value). A screenshot reads the image back from the
    GPU; the next frame can be slow. The report marks those hitches
    as "after a recorder screenshot."
  - **Without a window,** the recorder is off. `-- --bf-record` on the
    command line turns it on (no screenshots then).
- **`BfDevMenu`** is the dev menu for the gamepad (R13,
  `rust/game/src/devmenu.rs`). The game stops while it is open.
  - **Open.** Hold Back (Select) for 0.6 s. A tap of Back is still the
    playtest mark. Keyboard: F10.
  - **Use.** The d-pad moves, A chooses and B goes back. On the
    "Seed" entry, left and right change the number, and A makes that
    zone. The keyboard uses the arrow keys, Enter and Backspace.
  - **Main page.** Fly camera, debug overlay, new zone, seed, "Save
    playtest" (it calls `save_playtest()` on the recorder), Tests and
    Close.
  - **Fly camera on the gamepad.** The left stick moves and the right
    stick looks. The right trigger goes up and the left trigger goes
    down. The right shoulder button is fast.
  - **Tests page.** A test flies the fly camera one circle (350 m
    radius, 40 m above the ground) in 4 legs of 8 s. The Benchmark
    does this on seeds 1, 7 and 42. The Draw distance test does it on
    seed 1 with the far plane at 300, 800 and 3000 m. Push B to stop a
    test. After a test, the old zone comes back.
  - **Results.** Each flight writes a "test" event into the report and
    the numbers into `session.json` (`tests`): for each leg, fps, the
    average, P95 and max frame time, draw calls and primitives.
  - **A test without a window.** `godot --headless --path game
    -- --bf-record --bf-run-test=benchmark --bf-test-fast` runs a
    test with short legs and quits. A headless run has no real frame
    times and no draw calls.
- **`main.tscn`** is the start scene: atmosphere, terrain, bear,
  cameras, overlay, recorder and dev menu.

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

## Send a playtest (Mikey)

1. Play. When something looks or feels wrong, tap Back on the gamepad
   (keyboard: F9). "Mark 1 saved." shows on the screen.
2. Quit the game (close the window).
3. Open the `playtests` folder next to the game. Attach the newest
   zip to the chat or to the next Claude Code session.
4. Optional: write notes about feel and taste. You can name marks,
   for example "at mark 3 the bear looked strange."

If the game crashed, start it one more time and quit. The game then
makes the zip of the crashed session.

## Controls

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move | W, A, S, D or the arrow keys | left stick |
| Jog | hold Shift | hold the right shoulder button |
| Jump | Space | bottom button (A) |
| Climb | move into a steep surface | move into a steep surface |
| Climb up, down, left, right | W, S, A, D | left stick |
| Hop up a wall (uses stamina) | Space | bottom button (A) |
| Jump off a wall | Space with S (away from the wall) | A with the stick away from the wall |
| Let go of a wall | C | B |
| Turn the camera | mouse | right stick |
| Release the mouse | Esc (click the window to hold it again) | |
| Fly camera | F2 | dev menu (hold Back) |
| Debug overlay | F3 | dev menu |
| New zone | F4 | dev menu |
| Dev menu | F10 | hold Back/Select |
| Playtest mark | F9 | tap Back/Select |

All tuning values of the bear and the camera are in the Inspector
(select `Bear` or `FollowCamera`). The values of the animation are on
`Bear/Model`. The grip of each ground type is on `Terrain`. The
movement maths is in `rust/game/src/motion.rs` and
`rust/game/src/climb.rs`, with unit tests. The animation maths is in
`anim.rs` and `spring.rs`, with unit tests.

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
- Recorder test without a window:
  `godot --headless --path game --quit-after 1500 -- --bf-record`.
  The zip goes to the Godot user folder (`playtests/`).
- Load test without a window:
  `godot --headless --path game --quit-after 300`. On a new clone, run
  `godot --headless --path game --import` one time before this.
- CI (`.github/workflows/ci.yml`) runs `fmt`, `clippy` and `test` on
  each push. CI does not start Godot.
- The game build (`.github/workflows/game-build.yml`) builds the Rust
  library for Linux and Windows. Then Godot exports the game with
  `export_presets.cfg`. A smoke test starts the Linux build headless
  and checks that it makes a zone. A second run with `--bf-record`
  checks that the recorder writes a zip with `report.md` next to the
  game. The zips go on the `game-latest` release.
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
- **The dev menu opens after a 0.6 s hold of Back** (S-113). The
  recorder takes a tap of 0.4 s or less as a mark, so a hold between
  0.4 s and 0.6 s does nothing. Nobody tried a real gamepad
  [unverified]. A tap of Back while the menu is open is a mark if it is
  short.
- **The tests fly the fly camera** (S-113). The path is a circle of
  350 m radius and 40 m height around the centre of the zone. It is not
  the path of a real player. The draw distance test changes the far
  plane of the camera only. The Terrain3D draw range and the visibility
  ranges stay as they are (S-118).
- **All keys are fixed in code.** They are not in the Input Map, thus
  the player cannot change them.
- **Jog.** The bear walks at 1.4 m/s. It jogs at 3.2 m/s while Shift
  is down. The jump is 0.55 m high.
- **Camera.** Third person, 1.8 m behind the bear. The camera avoids
  the terrain only. It does not avoid other objects yet.
- **Start point.** The flattest place within 200 m of the centre. This
  can be a hilltop.
- **The walk is a small run** (S-120). The legs are 14 cm long, and
  the bear walks at 1.4 m/s. Thus each foot is on the ground for only
  about 30% of a stride, and the bear bounces from foot to foot. It
  makes about 8 steps each second at the walk and about 10 at the jog.
  A slow bear (stick pushed a little) makes a real walk. Mikey: is
  this too busy, or is it right for a toy?
- **The size of the movements** (S-120). The bounce, the waddle, the
  arm swing, the squash and the wobble of the ears and the hump are
  first guesses. Nobody saw them move in the Mobile renderer
  [unverified]. Mikey tells Claude what feels wrong.
- **The feet tilt with the legs.** A leg is one stuffed piece with no
  ankle. A foot in front of the hip stands on its heel, and a foot
  behind the hip stands on its toes.
- **Sand does not slow the bear.** On sand, the feet sink and slip,
  but the speed stays the same. A slower bear is a change to the
  movement rules, so Mikey decides it.
- **The animation runs in the physics step** (60 times each second),
  as the movement does. A screen with more than 60 Hz shows no extra
  steps.
- **The wall pose did not change.** On a wall, the belly is on the
  wall, the arms turn up at the shoulders, and the arms and legs swing
  a little with the climb distance (S-122).
- **The look of the bear** (R17). The fur is honey tan, the pads are
  pale cream, and the thread is almost black. Mikey has not seen it
  in the Mobile renderer yet [unverified]. The screenshots came from
  the Compatibility renderer.
- **Mesh detail.** The fur parts use mesh cells of 11 mm
  (`mesh_cell_m` on `Model`). This gives about 10 000 triangles. This
  is more than a PS1 character, but it is small for the GPU
  [unverified on Mikey's laptop].
- **The ears have no visible seam ring.** A ring around an ear base
  looks wrong, so the ear seam has only a position.
- **Climbing has no grab button.** The bear climbs when it pushes
  into a steep surface. The let-go button is C or B.
- **Hop, jump off and jog costs** (S-117). Jump on a wall hops up. To
  jump off, hold the stick away from the wall while you jump. A hop
  costs 4% of the stamina, and a jog costs 5% each second. These
  numbers are first guesses (`climb_hop_*`, `jog_stamina_seconds`).
  The jog uses the same stamina as the climb, so a long jog leaves
  less for a wall. Mikey: does this feel right?
- **Climb values.** The climb speed is 0.7 m/s. The stamina lasts
  30 s on a vertical wall and about 100 s on a slope of 50 degrees.
  This is too little to climb a full mountain without a rest.
- **Grip follows the colour.** Rust rock is firm, ash rock is
  slippery, and riverbed sand does not hold. Thus, the player can see
  the grip. Steep sand banks can stop the bear from leaving a
  riverbed there.
- **A fast bear cannot grab.** Above 6 m/s, the bear does not grab a
  wall. A bear that slides off keeps its speed.
- **A jump off only lets go.** A jump up the wall that uses stamina
  (as in Breath of the Wild) does not exist yet.
- **The camera does not change for the climb.**
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
- **The recorder is always on.** It cannot be turned off from the
  game (only with `enabled` on the `Recorder` node).
- **A hitch is a spike.** A frame is a hitch if it takes more than
  50 ms and more than 2 times the average frame of the last second.
  On a slow computer, each frame is longer than 50 ms; the second
  rule stops an event in each frame. "Low fps" events show the slow
  parts.
- **A tap of Back is shorter than 0.4 s.** A longer hold is free for
  the dev menu (S-113).
- **Hitches during a zone load do not count.** From the start until
  1.5 s after a new zone shows, hitches are "loading." The report
  counts them but does not list them as the worst hitches.
- **Paths are removed.** In the copy of the Godot log in the zip, the
  game folder, the user folder, the home folder and the user name
  become `<game>`, `<user data>`, `~` and `<user>`.
- **Frame cost.** The atmosphere has no measured cost. The parts that
  can cost time are the dust (300 particles, small) and
  the shadows (2 cascades, 100 m).
