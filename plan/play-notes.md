# Play notes

Mikey's notes from play sessions. The newest section is at the top.
Claude sessions use these notes to change the default values.

## 2026-11-14, S-125: playtest zip, build 32a0269 (30 min)

Mikey sent the zip only: no marks and no notes. Claude read
`report.md`. The facts below are from the report.

**The computer is not the target.** This laptop has an NVIDIA GTX 1650
(a discrete GPU), an i5-10300H and 7.6 GB RAM, at 144 Hz with vsync.
The speed target is integrated graphics (R5). A playtest on Mikey's
Vivobook (Intel integrated graphics) is still necessary (S-137).

**Speed on the GTX 1650**
- Average 139 fps (vsync at 144 Hz), median frame 7.1 ms. GPU render
  time about 3 ms. Draw calls 130 on average, at most 175.
- Benchmark: about 144 fps on seeds 1, 7 and 42.
- Draw distance test (seed 7): 300 m: GPU 1.3 ms, 112k primitives;
  768 m (smog): GPU 2.9 ms; 3000 m: GPU 3.1 ms. The smog cut helps a
  little; 300 m helps much more [the look at 300 m is not tested].
- "Script time, process" is 15.6 ms on average, higher than the
  median frame. This number needs a check (S-136).

**The main problem: long freezes**
- In the first minute, at the start point of seed 1: frames of 5.0 s,
  5.2 s, 3.0 s and 1.7 s, and 3 fps for 3 s. The bear stood still.
- After that: a freeze of about 950 ms every few minutes (17 times in
  30 min), on all seeds, also in the benchmark flights. Some are
  marked "(loading)" but no zone was loading.
- The Godot log has no errors.
- S-136 finds and fixes the cause.

**Obstacles (from the counts)**
- Rolling stones: 100 tells, 0 hits. The start point of seed 1 is near
  stone group #67, which tells every 13 s, but the stones never reach
  the bear.
- Launch vents: 4 launches, 3 missed the ledge, 1 reached it. Each
  miss ended in a hard tumble (16 m/s) that tore off both legs.
- Ash crust: 6 falls of 6. Gust: 4 pushes. Push vent: 6 hits.
  Updraft: 6 rides. Stepping stones: 1 cross, 2 falls. Sinking sand:
  5 sinks, 4 stuck, 4 escapes.

**Tumbles and limbs**
- 13 tumbles from hard landings, the fastest at 24.9 m/s. Getting up
  takes 3.1 s on average, at most 6.0 s.
- Legs come off often: left leg lost 7 times, right leg 6 times. Many
  of these came from launch-vent misses.

**Autoplay tour on this laptop:** pass. Seed 42 missed the climb task
(no grab in 20 s) and the updraft task.

## 2026-11-14, S-116: climb, smog and the new bear (Linux build)

**Climbing (for S-117)**
- The climbing speed is OK. The stamina is OK. When it runs out, that
  is part of the fun. Do not add more stamina or rest ledges yet.
- Ash rock and sand need more testing, but they feel OK for now.
- The jump off is OK. The bear climbs onto ledges well. Mikey saw no
  climbing problems. The camera is good during the climb.
- Wanted (R20, R21): a jump up the wall that costs stamina; the jog
  costs stamina too; a let-go button.

**Smog and sky (for S-118)**
- The smog, the distance, the sun, the sky and the colours are good
  for now. The dust particles are OK.
- Mikey asks: does the short view distance really make the game
  faster? He sees no difference. (Answer: not yet. The camera draws
  up to 3000 m, behind the smog. S-118 ties the draw distance to the
  smog and measures it.)
- Later: a real particle system that reacts to the world (wind,
  different materials), not only the simple dust. See `inbox.md`.

**The bear (for S-121)**
- The bear looks OK, and it is easy to see far away.
- The lines around the limbs and the neck look strange.
- The bear may be too bright. It must look like a normal, fluffy
  teddy bear.
- Wait for the new animation (S-120) before larger changes to the
  model.

**Speed**
- Mikey did not report numbers. From now on, the playtest recorder
  (S-123, R22) records them.

## 2026-11-14, S-111: the first start (Linux build from GitHub)

- Movement, camera and world feel good enough for now. No tuning is
  necessary.
- Mikey plays with a gamepad. He wants to sit and use only the
  gamepad. Button combos or a dev menu are OK.
- Climbing, falling apart, the zone edge and the long-term world: see
  `decisions.md` (R11 to R14) and `vision.md`.
