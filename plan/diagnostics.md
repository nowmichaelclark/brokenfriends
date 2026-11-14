# Diagnostics: the playtest recorder (R22)

Status: the recorder is built (S-123, `rust/game/src/recorder.rs`,
`game/README.md`). The dev menu with "Save playtest" and the "Tests"
page is built too (S-113, `dev_menu.rs`, `test_runner.rs`). Autoplay
is built (S-133, `autoplay.rs`). Not built yet: replay (S-124). A hitch must also be 2 times
longer than the average frame of the last second (see
`research/playtest-recorder.md`). Spec from 2026-11-14. Mikey's rule (R22): Claude must not ask Mikey
technical questions about a play session. The game records what
Claude needs. If a new technical question comes up, a session adds a
probe or a test to the recorder instead of asking Mikey.

Mikey only plays, presses a mark button when something looks or feels
wrong, and sends one zip file. He can add notes in his own words about
feel and taste. Those are the only questions for him.

## What the game records (always on, in the release build)

**At the start of a session** (`session.json`):
- the build commit (from `BUILD.txt`), the date and the time
- the hardware: CPU name and core count, RAM, GPU name and vendor,
  graphics API and driver version, the OS, the screen size and the
  window size
- the renderer, the render scale, the vsync mode and all project
  settings that change speed
- the seed and the zone generation time

**Each second** (`samples.jsonl`, one line for each second):
- fps; frame time average, p95 and max; CPU and GPU frame time (Godot
  viewport render-time measurement, if available)
- draw calls, primitives, objects, video memory, static memory
- the bear: position, speed, state (walk, jog, jump, climb, slide,
  tumble), slope, ground type, grip, stamina, limbs, the time of
  one animation step in microseconds (`anim_us`), and the smoke at the
  zone edge (`smoke_m`: depth in metres, `smoke_s`: seconds that count)
- the camera: position and direction
- the input: stick directions and buttons held

**Events** (`events.jsonl`), with the time and the place:
- **hitch:** a frame longer than 50 ms
- **low fps:** below 55 fps for more than 2 s
- **stuck:** the stick is held, but the bear moves less than 5 cm in
  2 s
- **climb loop:** grab and let go more than 4 times in 10 s at the
  same place
- **through the ground:** the bear is below the terrain height
- **state changes:** climb start and stop, jump, slide, tumble, death
  (`death:smoke:time`, `death:smoke:depth`), restart, `smoke_enter`,
  `smoke_leave`
- **new zone (F4):** the seed, the time to make it and the number of
  obstacles of each type
- **obstacles (R23):** bear events with the text
  `obstacle:<what>:<type>:#<number>`. `<what>` is `tell`, `hit`,
  `push`, `launch`, `fall`, `escape` (the bear left a crust in time,
  or got out of sinking sand), `cross` (the bear went over the stepping
  stones), `ride` (the bear was in an updraft), `sink` and `stuck`
  (the bear stood on sinking sand; the sink was full), or
  `launch_reached`, `launch_missed`, `launch_wall` (where a launch
  ended). The report counts them by type
- **errors and warnings** from the Godot log

**Screenshots** (`shots/`, JPEG, 960 x 540):
- one every 30 s
- one at each mark
- one at each hitch and each "stuck" event (at most one for each 10 s)

**Marks:** Mikey taps the Back/Select button (keyboard: F9) when
something looks or feels wrong. The game saves a screenshot, the last
10 s of samples at full frame rate, and a "mark" event with a number.
A short note on the screen confirms it: "Mark 3 saved." Later Mikey
can say "at mark 3 the bear looked strange," and Claude can see that
moment.

## The report

When the game quits (or from the dev menu: "Save playtest"), it
writes:
- `report.md`: a short summary for Claude. Hardware, build, play time,
  fps (average, 1% low, worst), the 10 worst hitches, a list of all
  events and marks, with the names of their screenshots.
- `playtest-<date>-<time>.zip`: all the files above and the Godot log.
  The target size is under 20 MB.

The files go into a `playtests/` folder next to the game. If that
folder cannot be written, they go to the Godot user folder. The game
deletes the oldest sessions when the folder is larger than 200 MB.

The files have no personal data: no names, no accounts, no file
paths outside the game folder.

## How Mikey sends a playtest

1. Play. Tap Back when something is wrong.
2. Quit the game.
3. Attach the newest zip from `playtests/` to the chat or to the
   Claude Code session.
4. Optional: write notes about feel and taste.

## Tests that Claude can add (the test menu)

The dev menu (S-113) has a "Tests" page. Each test runs by itself and
writes its results into the report. Mikey only starts it and waits.

- **Benchmark:** the camera flies a fixed path over seeds 1, 7 and 42.
  The result is fps, frame time and draw calls for each part of the
  path. The same path on each laptop gives numbers that we can
  compare.
- **Draw distance test:** the benchmark at three draw distances (for
  example 300 m, 800 m and 3000 m).
- More tests come later. When a session has a question, it adds a test
  here and a row to the agenda.

## Autoplay: the game plays itself

A bot plays the bear through the same input as the gamepad (virtual
stick and buttons). It does not move the bear directly, so it tests
the real controls. The recorder records an autoplay run like a normal
playtest.

**Modes**
- **Tour:** a fixed list of tasks for each seed. Walk to points far
  away, climb the nearest steep faces, jump at ledges, visit one
  obstacle of each type, go into the smoke edge, and try each limb
  set (two legs, one leg, no legs, one arm).
- **Wander:** random but seeded stick and button input for some
  minutes. It finds places where the bear gets stuck, falls through
  the ground, or shakes.

**Where it runs**
- **GitHub, after each push:** a short tour (about 2 minutes) on 3
  seeds, headless. The build fails if the game crashes, prints errors,
  or if the bear falls through the ground. "Stuck" and "climb loop"
  counts go into the build log.
- **GitHub, each night:** a longer wander on more seeds. The result
  goes into a short report on the release page.
- **Mikey's laptop (dev menu, "Tests"):** the same tour, with the
  real GPU. This gives speed numbers during real movement: climbing,
  obstacles, the smoke edge. Mikey only starts it and waits.

**What a bot cannot do:** it cannot say if something is fun, looks
good, or feels right. Mikey's notes stay the source for feel and
taste.

## Later: replay

The game can record the input of each physics tick. With the seed, a
cloud session can replay the exact play session headless and see the
same problem. Physics with Jolt may not repeat exactly, so the
recording also stores the bear's position each second, to check.
This is row S-124.
