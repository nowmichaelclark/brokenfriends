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
- Claude's notes: web builds need Godot's Compatibility renderer
  [unverified]; running Driftloom's core in a Godot web build is a
  risk to test early [unverified] (decision 0016).

## First questions

- Performance target: which cheapest phone must run it well?
