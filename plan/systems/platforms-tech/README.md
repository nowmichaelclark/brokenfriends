---
status: idea
last_reviewed: 2026-10-23
depends_on: []
feeds: [time-save, ui-ux, audio-music]
---
# Platforms and tech

## In charge of

The engine, the code project's layout, export targets, performance
budgets, the development workflow and the test scenes.

## Not in charge of

- Game rules: every other system.

## Seed notes (from the first chat, unconfirmed)

- 2D. Mobile first, browser too, maybe native Windows, Linux and Mac.
- Godot (decision 0004); built-in physics first, Rapier only if
  needed (decision 0005).
- Developed on Fedora 44; tested on Mikey's Android phone.
- Gym, zoo and museum test scenes as hidden debug scenes.
- No iPhone for now (decision 0015): Android and the browser first.
- One repo, `brokenfriends`: plans in `plan/`, the Godot project in
  `game/` (decision 0007).
- Web builds use the Compatibility renderer (V-003, verified). The web
  build uses Godot's extension-capable `web_dlink` engine so Driftloom's
  core can run in the browser (V-008, verified; spike 2).
- The game is built and running: `game/`, decisions 0019 and 0020.
- Mikey (2026-10-24): it should run well on his Moto G Power (2022)
  and on weaker phones.

## Performance practice (from Driftloom, in the build)

How the build stays light, borrowed from Driftloom's habits:

- Nothing is redrawn unless it changed. Detailed drawings (the tent,
  each part of a friend) are made once into a picture and then only
  moved; a part is drawn again only when it changes (a new eye, mud).
- Sounds are made ahead of time while the game starts, never in the
  middle of play.
- The game draws at 720 pixels wide however sharp the screen is
  (stretch mode `viewport`).
- Phones and the browser use Driftloom's lite mix.
- Measure, don't guess: `#perf` on the web address (or `--perf`)
  shows frame time, the music's cost and draw calls. Switches
  `nobg`, `nocreature`, `nohud`, `nomusic` and `empty` show what each
  part costs.

Measured 2026-10-24 in the test browser at the Moto's pixel density
with the processor slowed six times (the test machine draws in
software, so the numbers are only good for comparing):

| | Before | After |
|---|---|---|
| Draw calls in the tent | 220 | 62–78 |
| Frame time | 130 ms (7 fps) | 60–70 ms (15 fps) |
| An empty page, same machine | 34 ms | 34 ms |

On Mikey's Moto G Power in Chrome (2026-10-24, after the changes): a
steady 60 fps, 16.7 ms every frame, music 0.7 ms a frame, 65 draw calls,
on the tent's first screen.

In the Android app and on computers the music runs on Godot's audio
thread, not the screen's. The browser has no audio thread, and music
fed to it frame by frame crackled on Mikey's phone even at 60 fps, so
there each loop is made whole ahead of time and played as one sound.

## First questions

- Performance budget: what frame time on Mikey's phone counts as
  "runs well"? (Proposed: steady 30 fps or better.)
