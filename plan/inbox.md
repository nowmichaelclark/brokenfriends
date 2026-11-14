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
- Bear: other friends from the same code: a `Recipe` from a seed (head size, snout, arm and leg length, hump, colours).
- Bear: a lower-detail mesh (bigger `mesh_cell_m`) when the camera is far, if the triangles cost time on the laptops.
- Bear: water, mud or goo in the world that sets the wet value of the parts that touch it; wet parts dry slowly.
- Bear: a visible seam on the ears, if Mikey wants one (now only the neck, shoulders and hips have rings).
- A general particle system that reacts to the world: wind, different materials (sand, ash, stuffing, smoke), not only the camera dust (Mikey, S-116).
- Recorder: catch engine errors with `OS.add_logger` (Godot 4.5+) instead of reading the log file each second; it needs thread-safe code in godot-rust.
- Recorder: a frame-time graph (PNG) in each zip, so a hitch pattern shows at a glance.
- Recorder: a small "REC" dot on the screen, so Mikey knows that the recorder runs (only if he wants it).
- Before a public release, make the playtest recorder opt-in for other players (a setting, off by default). It records hardware data and screenshots. For Mikey's builds it stays always on (R22).
- Animation: footstep events (dust puffs, sounds later) from the moment a foot is planted (`anim.rs`, build order step 11).
- Animation: sand that slows the bear a little, if Mikey wants the ground to change the movement and not only the look.
- Animation: the head looks at things (the camera target, a wall before a grab, a friend), not only into the turn.
- Animation: a forward ray for the slope ahead, so the bear leans before it reaches a hill (`research/bear-animation.md`).
- Animation: run the pose in the render frame with interpolation, if a screen with more than 60 Hz shows a stutter.
- Animation: a debug view of the feet targets and the stride wheel in the F3 overlay or the dev menu.
- Crumbling ground that drops the bear into a cave (R23), when the caves exist (demo 2).
- Climb: let the climb movement use the holds of the hands (for example, no move up when no hand finds a hold), so the look and the rule agree.
- Climb: at a mantle, put the paws on the top of the ledge and pull the body up over them.
- Limbs: a lower collision capsule for a bear that lies (no legs), so it can crawl under low things.
- Limbs: a "pull, rest" rhythm in the speed of the one-arm drag, not only in the look.
