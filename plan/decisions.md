# Decisions

Only Mikey decides. A line goes in "Decided" only once he has said so,
with his name and the date. Old 2D decisions are in
`../archive/2d/plan/decisions/`. Those marked "carried over" still
hold.

## Decided

| # | Decision | Signed |
|---|---|---|
| R1 | Reboot: a 3D game for PC (maybe a mobile port later, not now), built from the ground up. The 2D version's ideas stay as inspiration. | Mikey, 2026-11-13 |
| R2 | Godot + Rust, with as much as possible in Rust from the start, so nothing has to be ported later. | Mikey, 2026-11-13 |
| R3 | The world is fully procedural and biome-first: the biome shapes the terrain. Terrain is smooth, with beautiful mountain shapes, and has gigantic caves deeper and bigger than Minecraft's. | Mikey, 2026-11-13 |
| R4 | Option A: a heightmap surface with clipmap LOD, plus a separate Rust volumetric cave layer, in finite zones. | Mikey, 2026-11-13 |
| R5 | 60 fps at 1080p on a laptop with integrated graphics. | Mikey, 2026-11-13 |
| R6 | The player is teddy-bear sized and everything is to scale. Tall enterable buildings come later; the natural world comes first. | Mikey, 2026-11-13 |
| R7 | The setting is post-apocalyptic and much hotter. Smog, pollution and dust have ruined the air, so the world has haze and smog, not plain fog. | Mikey, 2026-11-13 |
| R8 | The first goal is a playable demo, soon: explore one biome by walking, jumping and climbing. | Mikey, 2026-11-13 |
| R9 | Open source, MIT, free; no ads or microtransactions (carried over: 0018, 0023). | Mikey, 2026-10 |

## Proposed, waiting for Mikey

- **Renderer:** Mobile, with Compatibility as the fallback.
- **Terrain renderer:** Terrain3D. The fallback is our own clipmap.
- **Engine and crate versions:** Godot 4.7.1 and `godot` 0.5.x,
  pinned.
- **Demo zone:** 2 × 2 km. The biome is scorched highlands.
- **Camera:** third person.
- **Art:** no imported art for demo 1; everything is made in code.
