---
status: accepted
date: 2026-10-23
decided_by: Mikey, 2026-10-24
supersedes:
superseded_by:
---
# 0005: Built-in physics first, Rapier only if needed

## Decision

Start with Godot's built-in 2D physics. Switch to the Rapier plugin
only if a prototype shows a problem it solves.

## Context

An early plan (from DeepSeek) put the Rapier physics plugin in from
day one. The game's signature mechanics (tearing, sewing, weight,
squish) are scripted on top of any physics engine; none of them comes
from Rapier.

## Why

- Fewer moving parts while learning Godot, Linux and Android export.
- The physics engine is a project setting (`physics/2d/physics_engine`),
  so switching later shouldn't mean rewriting game code. Behavior can
  still differ: Rapier has no asymmetric collision layers (V-009).

## Alternatives rejected

- **Rapier from day one:** better stacking stability and 2D liquids,
  which may suit water creatures later, but one more plugin with its
  own limits.

## Consequences

If prototypes show jitter or unstable stacks, revisit with a new
decision.
