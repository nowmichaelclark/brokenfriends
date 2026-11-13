# Demo 1: the scorched highlands

Status: draft, 2026-11-13. Mikey chose the goal and the movement. The
biome and the details marked **assumed** are proposals for him to
confirm or change.

## The goal

You're a teddy bear walking, jumping and climbing through one
procedurally generated biome under a smog-choked sky. The demo is done
when Mikey plays for ten minutes and wants to keep exploring.

## What's in

**The zone.**
- 2 × 2 km, one seed. To a 40 cm bear that feels like 8–10 km.
- There's a dev key that regenerates the zone with a new seed.

**The biome: scorched highlands** (assumed).
- Ridged mountain ranges, 300–600 m from valley floor to peak.
- Eroded gullies, scree slopes and dry riverbeds.
- Bleached, cracked ground.
- Ochre, rust and ash colours.
- No water.

**The air.** It's hot, polluted and dusty.
- **Fog:** distance fog plus a low height fog, tinted yellow-brown
  rather than grey.
- **Sun:** a pale disc you can almost look at.
- **Distance:** mountains a few hundred metres off fade to flat
  silhouettes.
- **Particles:** dust and ash drift past the camera.
- **Heat:** a faint heat shimmer near the ground (assumed, cut it if
  it costs too much).
- **Colour grading:** a washed-out look.

**The teddy.**
- A placeholder bear built from simple shapes in code: body, head,
  ears and limbs.
- Third-person camera (assumed) that orbits with the mouse or right
  stick and doesn't clip into terrain.

**Movement.**
- **Walk.** Holding a key to jog is assumed.
- **Jump**, with coyote time and a jump buffer so it feels forgiving.
- **Climb:**
  - Press toward a steep surface to grab it, then move across it.
    Mantle over ledges.
  - Assumed: any slope steeper than about 50° is climbable rock.
  - Assumed: stamina limits climbing, then you slide off.
  - Assumed: falls don't hurt, because a teddy is soft.

**Ground scatter.**
- Rocks, pebbles and dead shrubs, made procedurally in Rust and drawn
  with MultiMesh.

**Debug.**
- An F3 overlay shows FPS, frame ms and draw calls, with the seed
  shown.

## What's out (later demos)

- Caves (demo 2).
- Other biomes and blending between them.
- More than one zone.
- Water.
- Friends and care.
- Buildings.
- Saving.
- Music and sound (Driftloom comes later).
- Menus.
- Any imported art (assumed: everything is made in code, as in the 2D
  build).

## Done when

- [ ] Mikey can walk, jump and climb across the whole zone without
      getting stuck or falling through.
- [ ] Mountains read as mountains through the haze: ridges, gullies,
      silhouettes.
- [ ] 60 fps at 1080p (render scale 0.75) on Mikey's laptop with
      integrated graphics, or a list of what's in the way.
- [ ] A new seed makes a different but just as good zone in under
      10 seconds.
- [ ] Mikey's ten-minute play, written up in `plan/play-notes.md`.

## Questions for Mikey

1. Is scorched highlands the right first biome? The runner-up is
   mesa/badlands with terraced cliffs.
2. Climbing: should it use stamina, and can everything steep be
   climbed or only rock?
3. Should there be a jog or sprint, or walking only?
4. Should there be fall damage? (Assumed none.)
