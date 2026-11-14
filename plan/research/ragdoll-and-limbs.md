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

## What S-114 found and built (2026-11-14)

Facts about Jolt in Godot (from the Godot source and docs, read on
2026-11-14 on `master` [unverified for the 4.7.1 tag]):
- The Jolt module supports the five Joint3D types. `ConeTwistJoint3D`
  becomes a Jolt `SwingTwistConstraint`. The twist axis is the local X
  axis of the joint. `swing_span` and `twist_span` work. Bias,
  softness and relaxation are ignored
  (https://docs.godotengine.org/en/latest/tutorials/physics/using_jolt_physics.html).
- `PinJoint3D` is a ball joint with no limits.
- `continuous_cd` gives Jolt "LinearCast": a shape cast along the
  motion. Fast turns can still go through thin things.
- Jolt says that bodies should be 0.1 to 10 m long. The parts of the
  bear are 0.03 to 0.2 m. The project setting
  `physics/jolt_physics_3d/collisions/collision_margin_fraction`
  helps small shapes. The headless test showed no problem
  [unverified on Mikey's laptop].
- Layers: two bodies collide if the mask of one has the layer of the
  other (the same as Godot Physics).
- `exclude_nodes_from_collision` (default true) stops a collision only
  between the two bodies of one joint.

Open-source examples (MIT, study only, we did not copy code):
- Drop-In Ragdoll (https://github.com/debugalot/drop-in-ragdoll):
  makes rigid bodies and joints from the pose at each fall. This is
  our method too.
- DanilKhmyrov/godot-active-ragdoll: cone and hinge joints with motors.

What S-114 built, with our own code:
- The ragdoll has 6 bodies (body, head, four limbs) with cone-twist
  joints. The capsule sizes come from the part meshes (`tumble.rs`).
  The mass comes from the volume (120 kg per cubic metre, about
  0.4 kg in total). The parts do not collide with each other (layer
  5, mask 1).
- A hit is the change of velocity of a limb body in one physics step,
  minus gravity. This needs no contact signals.
- The dangling limb is a chain of 6 Verlet points (`verlet.rs`).
- A lost limb gets more damping (turn 6/s, speed 0.6/s). Without it, a
  capsule rolled 12 m down a steep slope and did not stop.
