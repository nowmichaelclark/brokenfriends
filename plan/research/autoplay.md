# Autoplay: references (S-133)

These notes are for the autoplay bot (`diagnostics.md`, "Autoplay").
The code is in `rust/game/src/autoplay.rs` and
`rust/game/src/playtest/autoplay.rs`.

## Sources

- **Chickensoft GodotTestDriver** (MIT, C#). It runs in the game and
  sends real `InputEventJoypadButton` and `InputEventJoypadMotion`
  events (`PressJoypadButton`, `MoveJoypadAxisTo`, `HoldActionFor`).
  This is the same method as ours. Its tests are scripts; they do not
  search the world. Use it for ideas only.
  https://github.com/chickensoft-games/GodotTestDriver
- **Unity Automated QA, "Game Crawler"** (preview package, Unity
  terms). A monkey tester: it does random actions and stops when it is
  stuck. The docs do not say how it finds "stuck". It looks abandoned.
  https://docs.unity3d.com/Packages/com.unity.automated-testing@0.6/manual/GameCrawler.html
- **EA SEED, "Augmenting Automated Game Testing with Deep RL"**
  (Bergdahl et al., CoG 2020, paper only). Agents that learn explore a
  map for coverage and find places where a player gets stuck. The cost
  is training time. Too much for us now, but a coverage goal is a
  good idea for later. https://arxiv.org/abs/2103.15819

## Facts that the bot uses

A subagent checked these in the Godot and Terrain3D source code.

- **Virtual joypad input.** `Input.parse_input_event()` with a joypad
  event sets the axis and button state of device 0, also if no
  joypad is connected. `use_accumulated_input` is on, so the bear sees
  the new state one frame later. This also works with `--headless`.
- **`--fixed-fps 60`.** Each frame is 1/60 s of game time, and the
  engine does not wait for the real time. With 60 physics ticks each
  second, each frame has one physics step. A headless tour of 3 seeds
  (about 8 min of game time) took 43 s in the cloud container.
- **Terrain3D collision.** The default mode is `DYNAMIC_GAME`: the
  collision exists only within 64 m of the camera, and it changes in
  the physics step. After a teleport, the first ticks can have no
  collision. Thus the bot ignores a bear below the ground for 2 s
  after a teleport or a restart, and counts it as "after teleport".

## What we chose

- The bot does not move the bear. It sends the stick and the buttons
  (A jump, B let go, right shoulder jog). It changes a world direction
  into the stick with the yaw of the follow camera (`stick_for`).
- Only a teleport at the start of a task and the limb changes use the
  dev tools (`go_to`, `set_limb`), as F6 and the keys 1 to 4 do.
- The build fails only on things that are surely bugs: a crash, an
  error in the log, a bear more than 0.3 m below the ground (or put
  back from 0.5 m or more), a zone that does not load. A missed task (no grab, no death in the smoke) is only in the
  report, because the bot is simple.
