# Inbox

Write ideas for later here, one line for each idea. A Brain session
sorts them.

- When the world works, bring back ideas from the 2D version: friends with bodies that tear, get wet and grow mould; sewing; play instead of training; Driftloom music; delivery runs (`../archive/2d/plan/00-vision/`).
- The zone border is open: the terrain stops and the bear can fall off. Idea: a ring of high cliffs, or a slope down into thick smog.
- Scree: a talus pass (move material where the slope is more than about 40°) can make scree slopes below cliffs. The generator has no such pass yet.
- Ground detail smaller than 1 m (cracks, small steps) needs a detail texture or a displacement shader. The heightmap cannot show it.
- Dry lake beds are large flat areas. Idea: cracked-mud texture, salt crust, old boats.
- A Windows run of `cargo test` and one checksum compare with Linux will confirm "same seed, same world" on both laptops.
- A catapult that throws the bear to another zone (R14).
- The zone edge as a mix of smoke, goo and lava, in sectors that a seed selects (R14).
- A "strangeness" value for each zone (`vision.md`).
- Our own erosion simulation for avalanches, sand and runtime erosion (`decisions.md`, proposed).
- Look at TanTan's Rust projects (github.com/TanTanDev, Mikey's reference): `shrubbery` (deterministic procedural voxel trees with space colonization, MIT/Apache) for dead shrubs in S-108; `3d_celluar_automata` for cheap grid simulations (sand, goo, smoke edge).
- Climbing: a jump up the wall that uses stamina, as in Breath of the Wild (S-107 has only the jump off).
- Climbing: a "let go" button, and hands and feet that touch the wall with IK (`research/climbing.md`).
- Climbing: the follow camera can move back and look more from the side while the bear is on a wall.
- Climbing: places to rest on long slopes (small ledges in the generator), or stamina food, because one stamina ring is too little for a 400 m slope.
- Grip: a wet or oily ground type, and dust that falls where the bear slides.
- The follow camera avoids the heightmap only. When rocks or caves exist, it must use a ray cast as the climb does.
