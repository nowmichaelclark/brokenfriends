---
status: proposed
date: 2026-10-25
decided_by:
supersedes:
superseded_by:
---
# 0045: The first version is a dev release, with everything open

## Decision

The first version is a dev release for playtesting and trying things
out: as many body plans as can be built, and every mini game and place
easy to reach from the start.

## Context

Asked how many body plans the first version needs (Q-035), Mikey: "as
many body plans as possible for testing purposes. The 'First version'
is really a dev release meant for playtesting and trying things out.
Things will be different in later versions. E.g. A player won't have
access to every single minigame right at the start, but for testing,
it should be easy for me to access the minigames and have everything
easy to access from the start." (S-004, 2026-10-25)

## Why

- Body plans (four legs, insect legs, tentacles, floating, oozing)
  change movement and hitboxes; finding that out early shapes every
  interaction and mini game (Mikey's playtest notes, prototype 3).
- Testing is faster when nothing has to be earned first.

## Alternatives rejected

- **A few body plans, enough to test the loop** (the first playable's
  earlier "Out" list, Gate A): too narrow to learn how shapes move.

## Consequences

- `00-vision/first-playable.md`: the "full generated cast" line
  changes; body plans come in early, even ones later found only in
  some biomes.
- The build needs a quick way to reach every mini game and body plan
  (a dev menu; text is fine there, as in the physics test).
- Each body plan costs a rig, a way of moving, and front and back
  drawings (0031, 0037): a lot of work for one developer, so they come
  in one at a time.
- How a player later reaches mini games must square with "nothing is
  gated" (0010): Q-040.
