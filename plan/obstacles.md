# Obstacles (R23)

Status: draft, 2026-11-14. Mikey's idea (R23) and a first proposal.
Items marked **proposed** need Mikey's yes. S-127 built the framework
and four obstacles: ash crust, rolling stones, gusts and vents (push
and launch). S-128 built stepping stones, updrafts and sinking sand.
S-129 built the limb rippers: thorn snags, rock teeth and falling
rocks. `game/README.md` tells how they work now.

## Mikey's idea

- Procedurally generated obstacles of many kinds.
- Some obstacles can rip limbs off.
- Some obstacles help the player, for example vanishing platforms.
- Inspiration: *Fall Guys*, but made for one player in a procedural
  world.
- Obstacles can be part of the world itself. Example: a floor of
  blocks that breaks apart and drops the bear into a cave.

## Lessons from Fall Guys

From the article Mikey shared:
- **Readable from far away.** The player sees the danger and its
  timing before arriving.
- **Simple controls, physical reactions.** The obstacles push, launch,
  tilt and knock over. Falling over is part of the fun.
- **A good hit can help.** The same object can knock you back or, with
  good timing, launch you forward.
- **No pure luck.** Door Dash (doors that look the same, some fake)
  was called almost all luck. Avoid hidden information.
- **Many kinds:** spinning bars, tilting platforms, collapsing tiles
  (some shake first), launchers, punchers, fans, rolling objects,
  rising hazards.

## Our twist: the world is big to a small bear

The bear is 0.40 m tall. Normal things become obstacles at this scale:
a pebble that rolls down a slope is a boulder, a gust of wind is a
storm, a thorn bush is a forest of spikes, a crack in the ground is a
canyon. Thus, many obstacles can be natural and fit the world without
any buildings (R6: the natural world comes first).

## First obstacles (proposed, natural, no buildings)

**Dangers**
- **Crumbling ash crust:** thin ground over a hole. It shakes and
  cracks for a moment, then falls. Later it can drop the bear into a
  cave (demo 2).
- **Rolling stones:** stones that a slope or a trigger sends down a
  gully, one after another, with a sound and dust first.
- **Gusts:** strong wind in a narrow pass that pushes the bear.
  Visible dust shows the wind before it arrives.
- **Sinking sand:** it slows the bear and pulls it down. It is
  readable by its colour and ripples.
- **Steam or smog vents:** they blow at intervals. They push the bear
  away and get its fur wet (dirty steam).

**Help**
- **Vent launchers:** the same vents, used well, throw the bear up to
  a ledge.
- **Hardened-smog stepping stones:** platforms that appear and
  vanish in a rhythm. They make a path over a gap or sand.
- **Updrafts:** warm air over hot rock that slows a fall.

**Limb rippers (built in S-129)**
- **Thorn snags:** dead thorn bushes that catch the fur. A slow pull
  frees the bear; a fast pull tears a seam.
- **Rock teeth:** sharp rock edges in a slide path. A fast slide over
  them can tear a limb.
- **Falling rocks:** a rock that lands on a limb pins it. The bear can
  pull free and lose the limb, or wait.
- Later, with buildings: gears, fans, presses and other old machines.

## How the generator places obstacles (proposed)

1. After the terrain is made, find good places: gullies (rolling
   stones), passes (gusts), riverbeds (sinking sand), hot rock
   (vents), cliffs and gaps (launchers, stepping stones).
2. The seed selects which obstacle goes where. Fewer near the start,
   more further away.
3. **Fair rule:** there is always a way through or around. A tell
   (shake, dust, sound, colour) comes before each danger.
4. Each obstacle is data: a type, a place, a size, a rhythm and a
   difficulty. The same seed always gives the same obstacles.
5. The playtest recorder logs each hit, launch, fall and tear by
   obstacle type, so Claude can see which ones are fun and which ones
   are unfair (R22).

## Questions for Mikey (taste)

1. Are these first obstacles the right feel? What is missing?
2. Should the bear sometimes *want* to lose a limb, for example to
   escape a trap?
