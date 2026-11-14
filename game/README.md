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
  - **Holds on a wall.** Each hand and foot holds one place on the
    wall. A hand holds above its shoulder, and a foot below its hip.
    When the body moves 5.5 cm away from that place, the limb steps.
    A fan of 4 rays (2 for a foot) looks for a new hold a little ahead
    in the climb direction. Each hit gets a score from its distance,
    how much it faces the bear, and its grip. The two hands step in
    turn, and so do the two feet.
  - **The body hangs from the hands.** It follows the hands a little,
    hangs lower while a hand steps, and its shoulders follow the line
    of the hands. With one arm, it hangs to the side of that hand. If
    no ray finds a hold (above the top of a wall), the hand reaches to
    the wall and tries again after 0.15 s.
  - **Cost.** One step takes about 7 microseconds in the cloud
    container. On the ground, it uses 2 ray casts at most while the
    bear moves, and none while the bear stands. On a wall, a step of a
    hand uses 4 ray casts and a step of a foot 2; a bear that holds
    still uses none. The playtest recorder writes the time (`anim_us`
    in `samples.jsonl`).
- **Limbs and gaits** (R12, R18, `rust/game/src/limbs.rs`). The bear
  knows which arms and legs it has. A table gives the gait, the speed,
  the jump and the climb limits for each set of limbs:

  | Legs | 2 arms | 1 arm | 0 arms |
  |---|---|---|---|
  | 2 | walk; full climb | walk (95%); climb up to 68 degrees | walk (95%); no climb |
  | 1 | hop (60%); jump 60% | hop (51%); climb up to 68 degrees | hop (42%); small jumps; no climb |
  | 0 | arm-crawl (30%); no jump | one-arm drag (18%) | wriggle (10%); no climb |

  - **Hop.** The one foot stands below the middle of the body. The body
    goes up 3.5 cm in each hop and leans over the leg. The arms go out
    for balance.
  - **Without legs,** the bear tips forward onto its belly and lifts
    its head. In an arm-crawl, the paws plant in front of the
    shoulders, pull, and step in turn. A one-arm drag pulls with one
    paw and rolls a little. A wriggle turns the body from side to side.
  - **Climb costs.** The climb stamina costs more with fewer limbs: one
    arm x1.6, one leg x1.15, no legs x1.3 (the factors multiply). A
    bear without arms lets go of a wall.
  - **Debug keys.** 1 to 4 remove or give back the left arm, the right
    arm, the left leg and the right leg. 0 gives back all limbs. A lost
    limb is hidden. Code calls `set_limb("LeftArm", false)` and
    `restore_limbs()` on `Bear`.
  - **Recorder.** Each sample has the number of limbs and the gait.
    Each change is an event (`limb_off:LeftArm`, `gait:hop`).
- **`BfSmokeEdge`** is the wall of smoke around the zone (R14, R16,
  `rust/game/src/smoke_edge.rs`). The seed and the time set its shape
  (`bf-worldgen`, `smoke.rs`).
  - **Shape.** Waves around the circle set the front of the wall (760 m
    from the centre, plus or minus 170 m), its height (60 to 320 m),
    the width of its band (80 to 260 m) and how thick it is. The front
    moves in and out slowly: each wave moves it at 0.35 m/s at most. A
    new zone (F4) makes a new wall.
  - **The rule** (`smoke_dose.rs`). A timer counts the seconds that the
    bear is in the smoke. At 12 s, or at a depth of 45 m, the bear dies
    and starts again at the start point. Outside the smoke, the timer
    goes down twice as fast. A bear that stands higher than the top of
    the wall is not in the smoke. The start area is always clear.
  - **The fog.** In the smoke, the fog gets darker and thicker. It
    follows the danger, so the screen warns the player before the
    death. `BfAtmosphere` makes the fog (`set_smoke_amount`).
  - **The picture.** Two sheets and 96 billboard cards. The cards are
    big soft puffs that face the camera. A small noise image (made in
    Rust) gives the wisps. Rust sends the shape to the GPU as a float
    texture, 20 times each second. The wall is not hidden by the fog of
    the environment, so it shows from far away. It fades to the smog
    colour with its own density.
  - **Debug.** The F3 overlay shows the distance to the wall, or the
    time in the smoke. The recorder writes `smoke_m` and `smoke_s` each
    second, and the events `smoke_enter`, `smoke_leave` and
    `death:smoke:time` or `death:smoke:depth`. `kills` and `show_wall`
    on the node turn off the rule or the picture.
  - **Cost.** 3 draw calls. The cost is overdraw (transparent layers).
    Nobody measured it on the laptops [unverified]. `card_count` and
    `card_size_m` are the first values to cut.
- **`BfDevMenu`** is the dev menu (R13, S-113). On the gamepad, hold
  Back for 0.6 s. On the keyboard, push F10. The D-pad (or the arrow
  keys) chooses and changes, A (Enter) does, B (Esc) goes back. While
  the menu is open, the bear and the cameras ignore the input.
  - **Main page:** fly camera on or off, overlay on or off, new zone,
    seed (left and right change it, A makes that zone), view distance
    (from the smog, 300, 800 or 3000 m), the "Limbs" page, "Save
    playtest" and the "Tests" page.
  - **Limbs page.** A takes a limb away or gives it back. These are
    the keys 1 to 4 for the gamepad (`set_limb()` on the bear). "Give
    back all limbs" is the key 0 (`restore_limbs()`).
  - **Save playtest** calls `save_playtest()` on the `bf_recorder`
    group. The recorder writes the report and the zip, and the
    recording goes on.
  - **A tap of Back is a mark.** A hold of 0.6 s is free for the menu.
  - **The fly camera works on the gamepad.** The left stick moves, the
    right stick looks, the triggers go down and up, and the right
    shoulder button goes faster.
- **`BfTestRunner`** runs the tests of the "Tests" page (S-113). Each
  test flies a test camera along a fixed path: a closed polygon of 8
  sides around the centre of the zone (radius 500 m, 60 m/s, 25 m above
  the ground). The runner measures each frame and writes one row for
  each part of the path (fps, 1% low, p95, worst frame, draw calls,
  primitives, render time) into the `## Tests` section of `report.md`
  and into `tests/*.json` in the zip.
  - **Benchmark:** the path over seeds 1, 7 and 42.
  - **Draw distance test:** the path over seed 7 at the view distances
    300, 800 and 3000 m, and at the distance of the smog (the normal
    game).
  - **Stop.** B or Esc stops a test. The result so far is saved. After
    a test, the camera, the seed and the view distance go back.
  - **While a test runs,** the recorder takes no screenshots and marks
    its hitches as "loading", so they do not disturb the numbers.
  - **From the command line** (cloud, CI): `-- --bf-test=benchmark` or
    `-- --bf-test=draw-distance` starts the test when the first zone
    shows and quits the game at the end. Add `--bf-record` for a
    recording without a window.
- **`BfViewDistance`** ties the draw distance to the smog (S-118,
  `rust/game/src/view_distance.rs`). Godot makes the smog of an object
  at distance `d` as `1 - exp(-d * density)`. The game stops to draw
  at the distance where the smog is 99% (`smog_cutoff`). With the
  density 0.006, this is 767 m. The node reads the density twice each
  second and gives the distance to three places:
  - **Cameras.** Each camera of the game (group `bf_view_cameras`) gets
    the far plane: the distance times 1.05.
  - **Terrain3D.** `BfTerrain::set_draw_distance()` sets the number of
    mesh rings (`mesh_lods`). Each ring reaches twice as far as the
    one before it [unverified: the formula in `terrain_lods()`]. At
    767 m, Terrain3D makes 4 rings instead of 7.
  - **Props.** Each `GeometryInstance3D` in the group `bf_view_ranged`
    gets a visibility range that ends at the distance. Scatter (S-108)
    joins this group.
  - **The smoke wall** (S-115) is the exception. The far plane of a
    camera also reaches the far side of the wall, so the wall shows
    from far away (`keep_smoke_wall`). A forced distance (the tests)
    ignores this.
  - **Measured in the cloud** (software OpenGL at 4 fps, so the times
    are not valid for the speed). One camera 60 m above the start
    point: 4 rings, 216 000 primitives and 82 draw calls, against 6
    rings, 257 000 primitives and 96 draw calls without the smog link
    (-16% and -15%). In the draw distance test (8 parts, seed 7),
    the mean primitives for each frame were 313 000 at 300 m
    (3 rings), 372 000 at 800 m (5 rings) and 421 000 at 3000 m (7
    rings). The shot at 767 m shows no edge at the end of the view.
    The real numbers come from Mikey's laptops (S-125).
  - **Forced distance.** The dev menu and the tests set a distance
    (`set_forced_distance()`). `follow_smog` off makes the distance
    3000 m, as before S-118.
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
  300 dust particles move past the camera. In the smoke at the edge of the
  zone, the fog gets dark and thick. There is no heat shimmer
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
- **Obstacles** (R23, `plan/obstacles.md`). An obstacle is data: a
  type, a place, a size, a rhythm and a difficulty. The seed sets all
  of it.
  - **Places.** After the terrain is made, `bf-worldgen`
    (`obstacles.rs`) finds the places: flat ground, gullies, passes,
    hot rock, steep faces with a ledge, wide flat ground and riverbed
    sand. A zone has about 250 obstacles. The search takes about 0.15 s.
  - **Fewer near the start.** No obstacle is nearer than 18 m to the
    start point. The number and the difficulty grow with the distance.
    The nearest obstacle of each type is easy.
  - **Ash crust.** A pale, cracked disc on flat ground, 6 to 9 m
    wide, over a pit of 0.9 to 1.6 m. When the bear walks on it, it
    shakes for 0.9 s (hard: 0.45 s) and then falls. A fast bear gets
    off in time. The bear can climb out of the pit. A crust that fell
    stays broken until a new zone is made.
  - **Rolling stones.** At the top of a gully, 1 to 3 stones shake for
    1 s with dust, then roll down, one after another. A new wave comes
    each 8 to 14 s. A stone that hits the bear throws it down the
    slope. Stones start only if the bear is nearer than 90 m.
  - **Gust.** Wind along a narrow pass, in a rhythm of 7 to 9.5 s.
    Dust comes in for 1.2 s, then the wind blows for 2.2 s at 1.2 to
    2.6 m/s. The wind carries the bear. A bear in the air gets 1.5
    times the wind. A jog goes through, or the bear waits.
  - **Push vent.** A dark mound that leans to one side. It makes more
    and more steam for 1 s, then blows for 0.45 s. A bear in front of
    it is thrown to that side (3 to 5.5 m/s) and its fur gets wet.
  - **Launch vent.** A yellow mound at the foot of a steep face. It
    blows up. A bear on it flies to the ledge above (1.8 to 9 m). With
    no input, the bear keeps the direction to the ledge.
  - **Stepping stones** (help). A trench in wide flat ground: 16 to 22
    m long, 5 to 6 m wide and 1.6 to 2.4 m deep. 4 stones cross it, 1.6
    to 1.9 m apart. The stones take turns. The first and third are solid
    first. The second and fourth start half a cycle later. Thus, two
    stones next to each other are solid together for 1.1 s (easy) to
    0.5 s (hard). A bear can hop from one stone to the next then. A
    stone blinks and shakes for 0.7 s before it vanishes. A small dark
    ghost grows for 0.5 s before it is solid. A cycle takes 3.2 to 4.8 s.
    The bear can walk around the trench, or climb out of it.
  - **Updraft** (help). A column of warm air over very hot rock, 2.4 to
    3.4 m in radius and 6 to 10 m high. It is strong for 62% of a cycle
    (5.5 to 8.5 s) and weak (30%) for the rest. A bear in the air goes
    up to 2.2 m/s in it. Near the top the lift fades, so the bear hovers
    and then falls slowly. A bear on the ground gets no lift. Pale
    rising puffs show the column.
  - **Sinking sand.** A patch of riverbed sand, 2.8 to 4.2 m in radius.
    A bear that stands on it sinks by 0.09 m/s (hard: 0.17 m/s), up to
    0.3 m. A bear that walks sinks 0.4 times as fast. The sink slows the
    bear (12% at once, 65% at the full sink) and lowers its jump. The
    bear can always walk out. Sand comes back up at 0.2 m/s. The patch
    is dark with rings. Its collision goes down by the whole sink and
    the surface by one quarter of it.
  - **Wet fur** dries in 40 s.
  - **Code.** `ObstacleSim` (`obstacle_sim.rs`) moves the obstacles
    and has unit tests. `BfObstacles` (`obstacles.rs`) draws them and
    gives the wind and the hits to the bear. All tuning values are on
    the `Obstacles` node; the number of obstacles is on `Terrain`. The
    bear gets the lift and the slow down from `BfObstacles` with
    `set_lift` and `set_ground_slow`, as it gets the wind.
  - **Cost.** 4 draw calls for all puffs, stones, vent mounds and
    stepping stones, and 1 for each crust and each sand patch nearer than
    180 m. About 60 stepping stones have a collision body each; a stone
    that is not solid turns its shape off. At most 400 puffs show.
    Obstacles further than 170 m from the bear do nothing. The frame
    time is not measured on a laptop yet [unverified].
  - **Recorder.** Each tell, hit, push, launch and fall is a bear event
    (`obstacle:<what>:<type>:#<number>`). `report.md` counts them by
    type in "Obstacles". It also tells where each launch ended, and
    which bears crossed or fell off the stepping stones.
- **`main.tscn`** is the start scene: atmosphere, terrain, obstacles,
  bear, cameras, overlay and recorder.

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
| Dev menu | F10 | hold Back/Select for 0.6 s |
| Remove or give back a limb | 1 to 4, 0 | dev menu, Limbs page |
| Fly camera | F2 | dev menu, then the sticks and triggers |
| Debug overlay | F3 | dev menu |
| New zone | F4 | dev menu |
| Save playtest | | dev menu |
| Go to the next obstacle (dev) | F6 | dev menu (last item) |
| Playtest mark | F9 | tap Back/Select |
| Remove or give back a limb (debug) | 1, 2, 3, 4 | (dev menu, S-113) |
| Give back all limbs (debug) | 0 | (dev menu, S-113) |

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
- **Lost limbs are hidden** (S-122). A lost limb does not stay in the
  world yet. S-114 makes it a separate object.
- **A bear that lies keeps its tall collision capsule.** Only the
  visible bear lies down. The capsule stays 0.40 m tall, so a bear
  without legs cannot crawl under low things.
- **The gait speeds and limits are first guesses** (S-122): the
  numbers in the table above, the one-arm climb limit of 68 degrees
  and the stamina factors (on `Bear`). Mikey: does a bear on one leg
  or without legs feel right?
- **No jump without legs.** A bear without legs cannot jump. It can
  still grab a wall with its arms.
- **The drag has an even speed.** The one-arm drag looks like "pull,
  rest," but the movement code moves the bear at an even speed.
- **The holds on a wall are only a look.** The climb movement
  (`climb.rs`) does not read the holds. The hands can show a hold that
  the climb did not use.
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
- **The first obstacles are natural and have no sound** (S-127). The
  tells are for the eyes only: colour, shake, dust and steam. Mikey:
  are these four the right feel?
- **"Hot rock" is a noise field.** The generator has no heat data.
  Push vents are on flat ground where a slow noise field is high, so
  they come in groups. The ground there has no other colour yet.
- **A crust is large.** The heightmap has a sample each 1 m, so a pit
  cannot be smaller than about 4 m, and its lid is 6 m or more. For the bear, a crust is as wide
  as a pond.
- **The new help obstacles are first guesses** (S-128). Stepping stones
  take turns with a rhythm of 3.2 to 4.8 s; an updraft hovers the bear;
  sand sinks 0.3 m at most. Nobody played them yet. Mikey: do the
  stones feel fair? Is the updraft fun, or does it break the jumps? Is
  sand a danger, or only a slow patch? The bear cannot die in sand.
- **Stepping stones need a trench.** The terrain has no gaps, so the
  generator digs one. The first and the last stone are on the rims; they
  also blink. A stone in the ghost state has no collision.
- **Sand does not pull from below.** The bear stands on a lid that goes
  down. The bear does not sink into the real ground. At the full sink,
  the bear stays at the bottom of the pit (0.55 m deep).
- **Nothing hurts the bear yet.** A hit only throws the bear. Limb
  loss comes with S-114 and S-129.
- **Stones are not physics bodies.** They follow the slope of the
  heightmap and go through each other. A hit is a distance test.
- **Obstacle numbers are first guesses** (wind speed, throw speed,
  shake time, rhythm). Nobody played them yet. Mikey tells Claude
  what feels unfair or dull.
- **Same world on all computers.** The generator uses its own `sin`,
  `cos` and `exp` (`detmath.rs`), not the functions of the operating
  system. A test on Windows did not run yet [unverified].
- **The view distance is the 99% point of the smog** (S-118). Objects
  beyond it are 1% visible at most, so nobody sees them go. The low
  smog (below 90 m) makes more fog, so this value is safe. Mikey tells
  Claude if he sees a pop at the end of the view.
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
- **A tap of Back is shorter than 0.4 s.** A hold of 0.6 s opens the
  dev menu (S-113). A hold between 0.4 s and 0.6 s does nothing.
- **The menu does not stop the game.** The world goes on behind the
  menu, but the bear and the cameras ignore the input. A tap of Back in
  the menu also saves a mark.
- **The test path** (S-113) is a polygon with 8 sides at 500 m from the
  centre, flown at 60 m/s, 25 m above the highest ground in a few places
  along the path. The look is 12 degrees below the horizon along the
  path. Each run takes about 50 s plus the time to make the zone. The
  draw distance test uses only seed 7, to keep it short.
- **Hitches during a zone load do not count.** From the start until
  1.5 s after a new zone shows, hitches are "loading." The report
  counts them but does not list them as the worst hitches.
- **Paths are removed.** In the copy of the Godot log in the zip, the
  game folder, the user folder, the home folder and the user name
  become `<game>`, `<user data>`, `~` and `<user>`.
- **The smoke kills after 12 s or at 45 m** (S-115). These are first
  guesses (`max_seconds`, `max_depth_m` on `SmokeEdge`). A jogging bear
  (3.2 m/s) reaches 45 m in 14 s, so the time kills first. Mikey: is
  12 s too short or too long?
- **A square zone has a round wall.** The front is a radius from the
  centre, not a distance from the border. The wall is 40 m or more
  inside the border at all places. The corners of the zone are far
  behind the wall.
- **The wall is made for the zone size and seed only.** It does not
  know the terrain, except for the ground height that it reads once for
  each angle when a zone shows. A tall peak in the path of the front
  can poke out of the wall.
- **The wall time is the play time.** The wall starts at time 0 when the
  game starts. A new zone (F4) does not reset the time. The shape is
  the same for the same seed and the same time.
- **The wall has no sound and no particles of its own yet.** The cards
  are the only motion, except for the wisps in the texture.
- **The look of the smoke is a guess.** The pictures came from the
  Compatibility renderer with a software GL driver, not from the Mobile
  renderer on a laptop [unverified]. The wall is dark brown and fades
  to the smog colour from 300 m. Mikey tells Claude what he sees.
- **Frame cost.** The atmosphere has no measured cost. The parts that
  can cost time are the dust (300 particles, small) and
  the shadows (2 cascades, 100 m).
