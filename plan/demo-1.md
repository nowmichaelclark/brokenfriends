# Demo 1: the scorched highlands

Status: draft, 2026-11-13. Mikey chose the goal and the movement. The
biome and the items marked **assumed** are proposals. Mikey can accept
or change them.

## The goal

The player is a teddy bear. The bear walks, jumps and climbs through
one procedurally generated biome, under a sky full of smog. The demo
is done when Mikey plays for ten minutes and wants to continue.

## What the demo has

**The zone**
- The zone is 2 × 2 km, made from one seed. For a 40 cm bear, this
  feels like 8–10 km.
- A dev key makes the zone again with a new seed.

**The biome: scorched highlands** (assumed)
- There are ridged mountain ranges. Each peak is 300–600 m above the
  valley floor.
- Erosion makes gullies, scree slopes and dry riverbeds.
- The ground is pale and cracked.
- The colours are ochre, rust and ash.
- There is no water.

**The air** (hot, polluted and dusty)
- **Fog:** distance fog and a low height fog. The colour is
  yellow-brown, not grey.
- **Sun:** a pale disc. The player can almost look at it.
- **Distance:** mountains that are a few hundred metres away become
  flat silhouettes.
- **Particles:** dust and ash move past the camera.
- **Heat:** a small heat shimmer near the ground (assumed). If it
  costs too much frame time, remove it.
- **Colour grading:** the colours look faded.

**The teddy bear**
- The bear is a placeholder, made from simple shapes in code: body,
  head, ears and limbs.
- The camera is third-person (assumed). The mouse or the right stick
  turns the camera around the bear. The camera does not go into the
  terrain.

**Movement**
- **Walk.** Hold a key to jog (assumed).
- **Jump.** Coyote time and a jump buffer make jumps easy to time.
  Coyote time lets the bear jump a short time after it leaves an edge.
  A jump buffer keeps a jump press that comes a short time before
  landing.
- **Climb** (R11, like Breath of the Wild):
  - The bear starts to climb automatically when it pushes into a
    surface that is too steep to walk.
  - The bear moves across the surface in all directions.
  - Jump makes the bear jump off the surface.
  - The climb stops automatically when the surface is flat enough.
    At a ledge, the bear climbs up onto the top.
  - Climbing uses stamina. When the stamina is gone, the bear lets go.
  - Each surface has a grip value. On a slippery surface, the bear
    slides down slowly while it climbs. On a very slippery surface,
    the bear slides off completely. Fast slides keep their momentum
    and can make the bear tumble.
- **Coming apart** (R12): there is no fall damage. A hard fall or
  tumble can make the bear lose a limb. Lost limbs do not mean death.
  A bear without legs crawls. Demo 1 can do this in a simple way; the
  full version can come later.
- **Gamepad first** (R13): all actions work on the gamepad. The dev
  tools (fly camera, overlay, new zone) are in a dev menu.

**The zone edge** (R14)
- A wall of thick, dark smoke encircles the zone. Its shape comes from
  the seed and changes with time: it moves in and out, and it is
  sometimes high and thick, sometimes low and thin.
- The bear can go into the smoke a short distance. If it stays too
  long or goes too far, it dies. Then it starts again at the start
  point of the zone (R16).
- Goo and lava come later (assumed).

**Ground scatter**
- Rocks, pebbles and dead shrubs, made procedurally in Rust and drawn
  with MultiMesh.

**Debug**
- The F3 overlay shows fps, frame time and draw calls. It also shows
  the seed.

## What the demo does not have (later demos)

- Caves (demo 2)
- Other biomes, and blending between biomes
- More than one zone
- Water
- Friends and care
- Buildings
- Saving
- Music and sound (Driftloom comes later)
- Menus
- Imported art (assumed: code makes all art, as in the 2D build)

## Done when

- [ ] Mikey can walk, jump and climb across the full zone with a
      gamepad. The bear
      does not get stuck or fall through the ground.
- [ ] Through the haze, the mountains look like mountains: ridges,
      gullies and silhouettes.
- [ ] The game runs at 60 fps at full 1080p on Mikey's
      laptop with integrated graphics. If not, there is a list of the
      causes.
- [ ] A new seed makes a different zone of the same quality in less
      than 10 seconds.
- [ ] Mikey plays for ten minutes and writes his notes in
      `plan/play-notes.md`.

## The detail budget (R15)

The game renders at full 1080p. It does not need to look better than a
PS1 game upscaled to 1080p, but it does not copy the PS1 look. The
visual style of demo 1 is simple and is not final.

## Questions for Mikey

None open now.
