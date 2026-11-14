# Diagnostics: the playtest recorder (R22)

Status: the recorder is built (S-123, `rust/game/src/recorder.rs`,
`game/README.md`). Not built yet: the dev menu with "Save playtest"
and the "Tests" page (S-113, it calls `save_playtest()` on the
`bf_recorder` group), and replay (S-124). A hitch must also be 2 times
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
  tumble), slope, ground type, grip, stamina, limbs, and the time of
  one animation step in microseconds (`anim_us`)
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
- **state changes:** climb start and stop, jump, slide, tumble, death,
  restart
- **new zone (F4):** the seed and the time to make it
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

## Later: replay

The game can record the input of each physics tick. With the seed, a
cloud session can replay the exact play session headless and see the
same problem. Physics with Jolt may not repeat exactly, so the
recording also stores the bear's position each second, to check.
This is row S-124.
