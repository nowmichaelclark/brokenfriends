---
status: accepted
date: 2026-10-24
decided_by: Mikey, 2026-10-24
supersedes:
superseded_by:
---
# 0019: Start the Godot game now, alongside planning

## Decision

The Godot project in `game/` starts now, while planning continues,
beginning with the first five minutes from Mikey's recording.

## Context

`kit/PHASES.md` put code after the planning gates. Mikey wanted
something real to play while he's busy: "Can Claude go ahead start
building some things now so I can come back and play with something?"
He turned down throwaway web sketches: "I don't want playable sketches.
Can you not start the Godot game yet?"

## Why

- Playing it answers questions the docs can't (pillar 1, the feel of
  tearing and sewing, whether play is fun).
- The first five minutes was described in enough detail to build.
- The architecture is set up to grow: data separate from drawing, an
  event bus for music (0016), saving from day one.

## Alternatives rejected

- **Wait for the planning gates:** nothing to play for weeks.
- **A throwaway web sketch:** Mikey wanted the real game.

## Consequences

- Assumptions the build makes are listed in `game/README.md` and asked,
  never recorded as decisions without Mikey (CLAUDE.md).
- Phases 3 onward now refine a working build instead of starting one;
  `kit/PHASES.md` still applies to the planning docs.
- Risk: code shaped by guesses. Kept small by building only what the
  recording describes.
