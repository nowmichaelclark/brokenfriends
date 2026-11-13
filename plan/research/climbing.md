# Research: climbing (R11)

From the Research run of 2026-11-14. Check each licence again before
you copy code.

## Method to use

Climbing is a state of the Rust controller. It uses no physics bodies.

- **Start:** the player pushes into a surface steeper than about 50 to
  60 degrees, and a forward shape cast hits it within reach.
- **Move:** project the input onto the plane of the wall. Each frame,
  cast a short sphere along the negative normal to find the surface
  again. Use the average of 3 to 5 cast normals to make corners
  smooth.
- **Stop:** if the normal becomes flat enough to walk on, change to
  walking. If a cast above the head misses and a down-cast at the top
  hits flat ground, do a mantle (a timed move with IK hands).
- **Jump off:** turn climbing off for a short time after a jump, so the
  bear does not grab the wall again at once.
- **Stamina:** the drain depends on the slope. There is no drain while
  the bear does not move. A climb-jump costs a fixed amount.
- **Grip:** each surface has a grip value from 0 to 1, from its
  material or a procedural field (wet, oil, sand).
  `slide = max(0, gravity_along_wall * (1 - grip) - hold_force)`.
  Low grip: a slow slide. No stamina or very low grip: change to a
  falling or tumbling state that keeps the momentum.
- **Scale:** the bear is 0.40 m tall. Its reach is about 0.15 to
  0.2 m. Set all cast lengths for the bear, not for a human.
- **Hands and feet:** cast a ray from each hand and foot to the wall.
  Set the IK target to the hit point. Blend the IK weight from 0 to 1
  over a few frames. Godot 4.6+ has `IKModifier3D` and two-bone IK.

## Sources

| Source | Use | Licence |
|---|---|---|
| Catlike Coding, Movement 8: Climbing, https://catlikecoding.com/unity/tutorials/movement/climbing/ | The best reference for the core loop: wall-relative input, corners, overhangs, jump-off. Port the logic to Rust. | Code MIT-0. Do not copy the text (CC BY-NC-SA). |
| AitorSimona/Traverser, https://github.com/AitorSimona/Traverser | Ledge mantling without hand-placed markers. | Not confirmed. Study only. |
| conankzhang/procedural-climbing, https://github.com/conankzhang/procedural-climbing | BotW-style surface following in Unity. | Not confirmed. Study only. |
| wynterCG/grip_up, https://github.com/wynterCG/grip_up | Godot 4.7 + Jolt. Physics climbing (not our method), but a good render-only IK rig. | Not confirmed. |
| GDC 2017, "Breaking Conventions with BotW" | Design: walls are climbable; gliding gets you down. | Talk. |
| Vidqvist, thesis on BotW, section 3.2.2 | Stamina drain depends on steepness; no drain when idle. | Thesis. |
