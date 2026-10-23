---
status: proposed
date: 2026-10-23
decided_by:
supersedes:
superseded_by:
---
# 0004: Godot as the engine

## Decision

Build the game in Godot 4, scripting in GDScript.

## Context

Mikey asked whether anything beats Godot for a 2D, mobile-first game
that also runs in browsers and on desktop. Claude recommended Godot,
with Defold as the only serious alternative.

## Why

- Built-in tools for what this game needs: 2D skeletons with
  deformable skins, joints, shaders and particles [unverified, V-006].
- Exports to Android, iOS, web, Windows, Linux and Mac from one project.
- Large community and well documented; Claude Code works well with
  its text-based scenes and scripts.

## Alternatives rejected

- **Defold:** much smaller web builds and strong on mobile, but more
  bare-bones; 2D deformable skeletons usually mean the paid Spine
  tool [unverified, V-007]. May build iOS without a Mac
  [unverified, V-005].
- **Unity:** heavy, large web builds, history of licensing changes.
- **Phaser:** web-first; phones and desktop need wrappers.
- **GameMaker:** good at 2D, weaker for physics-heavy toys.

## Consequences

Web builds use the Compatibility renderer [unverified, V-003] and run
noticeably slower than native builds. Driftloom's Rust core would join
through Godot's extension system, which needs testing on web
[unverified, V-008].
