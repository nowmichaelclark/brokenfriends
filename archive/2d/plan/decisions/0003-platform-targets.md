---
status: accepted
date: 2026-10-23
decided_by: Mikey, 2026-10-23
supersedes:
superseded_by:
---
# 0003: Mobile first, also browser, maybe desktop

## Decision

The game is designed for phones first, must also be playable in a web
browser, and may later be ported natively to Windows, Linux and Mac.

## Context

Mikey's stated targets. They shape the engine choice, controls,
performance budgets and how music runs.

## Why

Mikey: "It's a mobile first game, but should also be playable in
browser and maybe ported natively as well to windows, Linux, Mac."

## Alternatives rejected

None discussed.

## Consequences

- Touch controls come first; mouse must also work.
- The browser build is the weakest platform for performance, so it
  sets the floor.
- Open: how much iPhone matters (iOS builds need a Mac
  [unverified, V-004]); see Q-003.
