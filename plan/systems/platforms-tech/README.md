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
- Web builds use the Compatibility renderer (V-003, verified). The
  standard web build has no GDExtension support, so Driftloom's core in
  the browser needs Godot's `web_dlink` templates and a spike (V-008,
  decision 0016).
- The game is built and running: `game/`, decisions 0019 and 0020.

## First questions

- Performance target: which cheapest phone must run it well?
