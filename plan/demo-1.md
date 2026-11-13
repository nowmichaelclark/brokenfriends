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
- **Climb:**
  - Push toward a steep surface to hold on. Then move across it. At a
    ledge, climb up onto the top.
  - All slopes steeper than about 50° are rock and can be climbed
    (assumed).
  - Stamina limits the climb. When the stamina is gone, the bear slides
    down (assumed).
  - Falls do not hurt the bear, because a teddy bear is soft
    (assumed).

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

- [ ] Mikey can walk, jump and climb across the full zone. The bear
      does not get stuck or fall through the ground.
- [ ] Through the haze, the mountains look like mountains: ridges,
      gullies and silhouettes.
- [ ] The game runs at 60 fps at 1080p (render scale 0.75) on Mikey's
      laptop with integrated graphics. If not, there is a list of the
      causes.
- [ ] A new seed makes a different zone of the same quality in less
      than 10 seconds.
- [ ] Mikey plays for ten minutes and writes his notes in
      `plan/play-notes.md`.

## Questions for Mikey

1. Is "scorched highlands" the correct first biome? The second choice
   is mesa/badlands, with terraced cliffs.
2. Climbing: do you want stamina? Can the bear climb all steep
   surfaces, or only rock?
3. Do you want a jog or a sprint, or only walking?
4. Do you want fall damage? (Assumed: no.)
