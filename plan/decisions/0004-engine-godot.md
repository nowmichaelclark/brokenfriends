---
status: accepted
date: 2026-10-23
decided_by: Mikey, 2026-10-24
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
  deformable skins, joints, shaders and particles (V-006).
- Exports to Android, iOS, web, Windows, Linux and Mac from one project.
- Large community and well documented; Claude Code works well with
  its text-based scenes and scripts.

## Alternatives rejected

- **Defold:** much smaller web builds and strong on mobile, but more
  bare-bones; 2D deformable skeletons usually mean the paid Spine
  tool (V-007: true; the free DragonBones route is a third-party extension). May build iOS without a Mac
  [unverified, V-005].
- **Unity:** heavy, large web builds, history of licensing changes.
- **Phaser:** web-first; phones and desktop need wrappers.
- **GameMaker:** good at 2D, weaker for physics-heavy toys.

## Consequences

Web builds use the Compatibility renderer (V-003, verified) and run
noticeably slower than native builds. Driftloom's Rust core would join
through Godot's extension system, which needs testing on web
[unverified, V-008].

## Sources

- Godot 4.7.1-stable class reference (`doc/classes/` in godotengine/godot): Skeleton2D, Bone2D, Polygon2D (bones and weights), PinJoint2D, ShaderMaterial, CPUParticles2D, GPUParticles2D all exist; docs "2D skeletons" describes the skin deformation. Checked 2026-10-25. V-006.
- Defold has no built-in bone editor; Spine support is the official `defold/extension-spine` (Spine runtime 4.2.xx; extension since Defold 1.2.193), and DragonBones is a third-party extension (defDragon). Esoteric's Spine Runtimes License requires each user to hold a Spine Editor license. Defold forum, Defold asset pages and Esoteric license page, seen through search results only, checked 2026-10-25. V-007.
