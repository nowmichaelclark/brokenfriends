# Research: tumbling and coming apart (R11, R12)

From the Research run of 2026-11-14. Check each licence again before
you copy code.

## Method to use

- **Always on: a fake ragdoll in Rust.** Each limb is a short verlet
  chain (a few points with fixed distances) with a spring toward the
  normal pose. This gives a floppy, stitched-doll feel at almost no
  cost. It is the Rain World method.
- **Real physics only when needed:** for a knock-down or a tumble, use
  a partial ragdoll on Jolt: `physical_bones_start_simulation()` with
  a list of bone names. The **Influence** property blends physics and
  animation. Use 6 to 10 capsule bodies. Turn self-collision off where
  possible. Freeze the ragdoll after it settles. Jolt is the default
  physics engine for new 3D projects since Godot 4.6.
- **Limb loss:** keep a bitmask of limbs on the bear. When a limb comes
  off, scale its bone to about 0 and spawn a cheap separate limb (a
  RigidBody3D, or a verlet limb that later becomes static). Then
  change the movement profile:
  - One leg missing: hop, with a shorter stride and a lower speed.
  - Both legs missing: crawl with the arms.
  - Arms missing: no climbing, or a "hug-climb" with a high stamina
    cost.
- Limb loss is a game mechanic: it changes the climb, grip and stamina
  rules.
- No measured numbers exist for Jolt ragdolls on integrated graphics.
  Measure in our scene.

## Sources

| Source | Use | Licence |
|---|---|---|
| Godot docs, Ragdoll system, https://docs.godotengine.org/en/stable/tutorials/physics/ragdoll_system.html | Partial ragdolls, Influence, cost for each bone. | Godot (MIT). |
| Rain World, GDC 2016 Animation Bootcamp, https://www.gdcvault.com/play/1023475/Animation-Bootcamp-Rainworld-Animation | Creatures as points with fixed distances: the fake ragdoll model. | Talk. |
| ImmersiveRPG/ExampleRagdoll, https://github.com/ImmersiveRPG/ExampleRagdoll | Active ragdolls with breakable limbs in Godot. | MIT |
| CBerry22/Active-Ragdoll (Godot 4), https://github.com/CBerry22/Active-Ragdoll---Physics-Animations-in-Godot-4.0 | Drive physical bones toward animated poses. | MIT |
| TjhaiME/Godot-Active-Rigid-Ragdoll, https://github.com/TjhaiME/Godot-Active-Rigid-Ragdoll | A wobbly, toy-like body from joints only. | MIT |
| PiCode9560/Godot-4-Active-ragdoll, https://github.com/PiCode9560/Godot-4-Active-ragdoll | Human: Fall Flat-style ragdoll. | MIT |
| DanilKhmyrov/godot-active-ragdoll, https://github.com/DanilKhmyrov/godot-active-ragdoll | A crawl modifier for missing limbs. | Not confirmed. Study only. |
| Thyke/Dismemberment, utkualkan4112/Dismemberment-System | Hide the bone, spawn a limb; health for each limb. | Not confirmed. Use the idea only. |
