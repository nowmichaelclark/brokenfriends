---
status: accepted
date: 2026-10-24
decided_by: Mikey, 2026-10-24
supersedes:
superseded_by:
---
# 0017: "Play", not "train"

## Decision

Creatures grow by playing, not training. The system once called
`training-minigames` is now `play`.

## Context

The pitch session preferred "play" over "train"; Mikey confirmed it
when asked whether to rename the system.

## Why

Mikey: "Train sounds like a gym; yes their stats can increase from
playing, but it is still largely playing through which the creatures
develop their skills or perhaps even learn new skills."

## Alternatives rejected

- **Keep "training" for the one system:** two words for one idea
  invites drift.

## Consequences

- `systems/play/` replaces `systems/training-minigames/`.
- Stats and skills still grow, and new skills may be learned, through
  play.
