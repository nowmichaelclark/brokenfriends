---
status: accepted
date: 2026-10-25
decided_by: Mikey, 2026-10-25
supersedes:
superseded_by:
---
# 0038: Music changes at moments that matter

## Decision

Music changes as in a console game, not on a timer: a new room, a
friend made well, a change of mood, the time of day, surgery, a death.
The changes also tell the player, quietly, that something changed or
went well. The music stays calm enough to sit and listen to.

## Context

In the sketch the music changed every 30 seconds or so (it cycled
Driftloom's test loops). Mikey: "the music should change at like
significant moments, not just like random moments... not necessarily a
normal mobile game, because sometimes mobile games will just change
music every 10 seconds."

## Why

- Fits 0032 (no text): music is one of the ways the game speaks.

## Alternatives rejected

- **Cycling tracks:** the sketch's way; it felt random.

## Consequences

- `audio-music`: which moments change the music, and how Driftloom's
  generator follows the friend, the mood and the time of day.
