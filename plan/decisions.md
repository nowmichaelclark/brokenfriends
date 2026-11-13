# Decisions

Only Mikey makes decisions. A line goes in "Decided" only after Mikey
agrees. Each line has his name and the date. The old 2D decisions are
in `../archive/2d/plan/decisions/`. The lines marked "from the 2D
version" are still valid.

## Decided

| # | Decision | Signed |
|---|---|---|
| R1 | Start again as a 3D game for PC. A mobile port can come later. Build it from zero. Keep the ideas of the 2D version for inspiration. | Mikey, 2026-11-13 |
| R2 | Use Godot and Rust. Put as much as possible in Rust from the start, so that we do not have to move code later. | Mikey, 2026-11-13 |
| R3 | The world is fully procedural. The biome comes first and gives the shape of the terrain. The terrain is smooth, with beautiful mountain shapes. Later, it has very large caves, deeper and larger than the caves of Minecraft. | Mikey, 2026-11-13 |
| R4 | Use Option A: a heightmap surface with clipmap LOD (level of detail), and a separate volumetric cave layer in Rust, in finite zones. | Mikey, 2026-11-13 |
| R5 | The game runs at 60 fps at 1080p on a laptop with integrated graphics. | Mikey, 2026-11-13 |
| R6 | The player has the size of a teddy bear. All objects have their real size. Tall buildings that the player can go into come later. The natural world comes first. | Mikey, 2026-11-13 |
| R7 | The setting is after an apocalypse. The world is much hotter. Smog, pollution and dust make the air bad. Thus, the world has haze and smog, not plain fog. | Mikey, 2026-11-13 |
| R8 | The first goal is a playable demo, as soon as possible. The player explores one biome. The bear walks, jumps and climbs. | Mikey, 2026-11-13 |
| R9 | The game is open source (MIT licence) and free. It has no ads and no microtransactions (from the 2D version: 0018, 0023). | Mikey, 2026-10 |
| R10 | Mikey plays builds that GitHub makes. He does not build on his laptop (it has little disk space). He tells Claude what he feels and sees. Claude sessions change the tuning values. | Mikey, 2026-11-14 |
| R11 | Climbing feels like Breath of the Wild and Tears of the Kingdom. The bear starts to climb automatically when it pushes into a steep surface. Jump makes the bear jump off. The climb stops automatically on a surface that is flat enough. Climbing uses stamina. The bear can try to climb slippery surfaces, but it can slide off. On a very slippery surface, it slides off completely. The slide can keep its momentum and make the bear tumble down. Some ragdoll physics is good, but the game must run on very low-end computers. | Mikey, 2026-11-14 |
| R12 | There is no fall damage as such. Instead, the bear can come apart as a real teddy bear would, for example lose limbs. Lost limbs do not mean death. A bear without legs can still crawl and climb. | Mikey, 2026-11-14 |
| R13 | Gamepad first. Mikey wants to sit and use only a gamepad. Button combos or a dev menu are OK. | Mikey, 2026-11-14 |
| R14 | The outer edge of a zone is a danger zone. The player can go into it a short distance. If the player stays too long or goes too far, the bear can die. Ideas: a thick cloud of dark or strangely coloured smoke that moves into and out of the world in procedural shapes; goo that traps the bear; lava that burns it; or a mix. It must be different from the circle of Fortnite. Later, a catapult can send the player to another zone. | Mikey, 2026-11-14 |
| R15 | The game is **not** going for a PS1 look. It renders at full 1080p. It does not need to look better than a PS1 game upscaled to 1080p, so it does not need high-resolution textures or many polygons. Performance aim: if a computer runs a well-made PS1 PC port or decomp (for example Silent Hill) at 60 fps and 1080p, it should also run Broken Friends. The visual style is still open. | Mikey, 2026-11-14 |
| R16 | For now, when the bear dies, it starts again at the start point of the zone. | Mikey, 2026-11-14 |
| R17 | The bear looks like an old, classic teddy bear (Mikey's reference photos: a jointed mohair bear with felt pads, a stitched nose and button eyes). It can tear, lose stuffing, and get wet fur. It feels soft, plush and a little furry, with simple shapes and parts that move easily. | Mikey, 2026-11-14 |
| R18 | The bear's animation is procedural. The engine makes the movement from the surface (for example how the bear climbs onto a wall) and from the limbs that the bear still has: with one leg it hops, with one arm it climbs less well and only on less steep surfaces. Mikey asks Claude to choose the method. | Mikey, 2026-11-14 |
| R19 | No heat shimmer. It did not look good, and the game is faster without it. | Mikey, 2026-11-14 |

## Proposed, Mikey must decide

- **Renderer:** Mobile. Compatibility if Mobile fails.
- **Terrain renderer:** Terrain3D. Our own clipmap if Terrain3D fails.
- **Versions:** Godot 4.7.1 and `godot` 0.5.x, both pinned.
- **Demo zone:** 2 × 2 km. The biome is scorched highlands.
- **Camera:** third person.
- **Art:** no imported art in demo 1. Code makes all art.
- **Visual style:** still open. A later Brain session proposes options
  inside the R15 budget.
- **Erosion filter:** keep the MPL-2.0 port for now. Write our own
  erosion simulation later, for the dynamic world (avalanches, sand,
  runtime erosion). That is a different tool from the filter.
- **Zone edge for demo 1:** the smoke edge only. Goo and lava come
  later.
