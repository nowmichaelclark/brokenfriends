# Vision: the world, long term

Status: idea, from Mikey, 2026-11-14. This is not a plan for demo 1.
Demo 1 is a natural world. Use this doc to keep later choices open.

## The direction

Mikey asks: what if *LSD: Dream Emulator* (PS1), *Death Stranding*
and *Minecraft* made one world? This is about the world, not the
gameplay.

- **From Minecraft:** a seed makes endless, different worlds. Each
  hill can hide something new, and caves go deep.
- **From Death Stranding:** travel over the land is the main
  activity. The shape of the ground matters. The views are wide and
  empty. Weather is a danger.
- **From LSD: Dream Emulator:** the world is like a dream. Colours,
  textures and rules can change. Strange things stand next to normal
  things. The mood is calm but a little wrong.

## Things Mikey wants later

- Very strange structures: land bridges, arches, objects like
  avant-garde statues, surrealist forms. They must be fun to climb and
  to look at.
- Different types of water: lakes, rivers, waterfalls and more.
- Strange colours, weather, gravity and patterns.
- Wild dynamic events: weather, waterfalls, erosion, blowing sand,
  avalanches, rocks that tumble down.
- Tall buildings that the bear can go into, at true scale.
- Zones joined by catapults: the bear is thrown to the next zone.

## How this fits the technology

- The SDF layer that makes the caves can also make arches, land
  bridges and statues. An SDF shape is a mix of simple shapes (balls,
  boxes, capsules) and noise, so a seed can make strange sculptures.
- Each zone can get a seeded "strangeness" value. At 0, the zone is
  natural. At 1, colours, gravity, sky and structures become strange.
  Demo 1 uses 0.
- Dynamic events need runtime changes to the heightmap. Terrain3D can
  edit its data at runtime [unverified: the cost of collision
  updates]. If this is too slow, our own clipmap (see `tech.md`) gives
  full control.
