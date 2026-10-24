---
status: building
date: 2026-10-25
tests: [friend-body, condition, care-touch, needs-growth, play, expeditions, time-save]
---
# Prototype 3: the first playable

## The question

Is the core loop fun at one camp, over days of real play: caring for
several friends, sending them on quests for what care needs, new
friends arriving, and friends dying?

## Smallest build

`00-vision/first-playable.md` (after Gate A), built as the steps below.
Each step is one session or a few, and is pushed so Mikey can play it.
The game stays a sketch (0021); design guesses go under "Assumed in
this build" in `game/README.md` and are asked, never decided.

**Every step is not done until:**
- its own "done when" holds, shown by a test or screenshots;
- the playthrough test ends `FLOW OK`, extended to cover the step;
- the web and Android builds pass on GitHub;
- speed holds: no frame-time or draw-call regression in `#perf` at 4x
  slowdown, and nothing heavy made mid-play (`platforms-tech`);
- `game/README.md` says what changed and what was assumed.

**Steps** (tick when done; the core loop is B1 to B6):

- [x] **B1. A camp of friends.** Several friends at one camp, chosen by
  tapping; saved and reloaded; time passes while away. The code says
  "friend", not "creature" (0022). *Done when:* three friends survive
  a quit and reopen, and a debug clock (`#days=N`) moves time on.
- [x] **B2. Change over time** (0028, 0029). Rot needs a source;
  untended wounds worsen; mold spreads; hunger speeds rot; a lost brain
  is death. Time away leaves traces: mud tracks, bugs, mold, a soaked
  friend sitting weakly. *Done when:* a test shows a wounded friend left alone dies
  and a healthy one lives, and screenshots show the traces.
- [x] **B3. Supplies run out** (0027). Stuffing, oil, thread and spare
  parts are counted and shown as things on a shelf, not numbers.
  Without them, the treatment that needs them can't be done (0042).
  *Done when:* using them up empties the shelf and blocks that
  treatment.
- [x] **B4. Parts** (0025, 0026). Any part comes off after several
  pulls; a donor hops or crawls; parts can die; fluid leaks where a
  part tore; head swap leads into sewing; a dead friend's parts go to
  a box. *Done when:* the playthrough tears a healthy arm off, gives it
  to another friend, and swaps a head.
- [x] **B5. Quests** (0034). A map: camp, a few places, two or three
  preset routes each, danger drawn on the map, the chance shown as a
  picture. A friend leaves for real minutes or hours and comes back
  changed: supplies, parts, wounds, sometimes a new friend; or dies out
  there (one or two parts come back). *Done when:* 20 quests on the
  debug clock give varied results that follow the friend's condition,
  skills and route.
- [x] **B6. Arrivals** (0034). New friends turn up with problems, now
  and then bringing a gift; the camp has a limit (0013). *Done when:*
  a simulated month brings arrivals and respects the limit.
- [ ] **B7. The surgery table** (0031). Surgery lying down in its own
  place, close enough to see stitches, front and back. *Done when:*
  reached from the tent, and sharp at full zoom.
- [ ] **B8. Generated friends, first cut** (0037). A few body plans
  with generated heads, ears, colours, fabric and proportions; front
  and back; thin friends wobble, plump ones bounce; a voice per
  friend. *Done when:* 20 seeds look clearly different, parts swap
  between any two, and speed holds.
- [ ] **B9. Play, second cut** (0033, pillar 6). The ball hits any
  part; a foot kicks it back; skills grow slowly; a skilled friend
  still fumbles. *Done when:* catching takes many throws to learn.
- [ ] **B10. No text** (0032). Hints go; what to do next is shown.
  *Done when:* Mikey plays the first five minutes with no words on
  screen.
- [ ] **B11. Music at moments** (0038). Music changes on a room change,
  a friend made well, a return, a death, the time of day; never on a
  timer. *Done when:* a test logs each change with its reason.
- [ ] **B12. Mikey plays for a few days**, and the result goes below.

Mikey plays after B6 (the core loop) and again after B11.

## Out of scope

Everything first-playable.md leaves out: more camps and biomes, the
ending, sharing codes, the friend maker for players, chores, freehand
routes, the full generated cast.

## How Mikey will judge it

Play for a few days, a few minutes or longer each time:
- Do you want to check on your friends again?
- Are quests worth sending friends on, and is the risk readable?
- When a friend died, could you see why? Did it feel fair?
- Does scarcity make choices interesting, or just annoying?
- What's the first thing you'd change?

## Result

Not played yet.

## What changed in the docs

Gate A (`reviews/2026-10-25-gate-a.md`) cut the first playable's scope.
