---
status: accepted
date: 2026-10-25
decided_by: Mikey, 2026-10-25
supersedes:
superseded_by:
---
# 0044: Any friend can play; its body changes how

## Decision

Every mini game can be played by any friend, tiny or huge, with or
without some parts. A friend's body (size, weight, arms, legs, seams,
wetness) and skill change how it plays, rarely whether it can.

## Context

Mikey: "we need a strong framework also. For instance a tiny friend and
a huge friend should both be able to play the same mini games. A friend
with no legs could still play too, but maybe some mini games wouldn't
be possible with no legs. Still, it would be fun if a friend with no
legs could at least try a skateboarding grinding mini game. But also,
we need to be thinking about the physics of the bodies." He went with
Claude's framework and physics test (2026-10-25).

## Why

- Surgery then matters outside the tent: which arm or leg a friend has
  changes what it's good at.
- Mini games ask for abilities (size, weight, grip, push, jump,
  balance, float), not parts, so new ones are cheap to add.
- The player times; the friend's skill widens the window and fumbles
  stay possible (0033).

## Alternatives rejected

- **Each mini game lists the parts it needs, and locks friends
  without them:** a grey "can't play" is no fun.

## Consequences

- Friends become soft ragdolls first (`prototypes/04-physics-test.md`).
- Every size needs mini games it's best at.
- When a body truly can't do a game (rare), the friend shrugs or plays
  a similar short animation and the mini game closes (Mikey, Q-032:
  "the minigame plays a short scene of the creature shrugging it's
  shoulders... and then the minigame closes out"). "Almost always a
  friend should be able to do a mini game, but there will probably be
  rare examples where it just won't be possible."
