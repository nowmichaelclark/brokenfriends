# Research: the bear's procedural animation and look (R17, R18)

From the Research run of 2026-11-14. Check each licence again before
you copy code. Costs are estimates, not measurements.

## The main choices

- **No skeleton.** Rust poses each part mesh directly (with
  `RenderingServer::instance_set_transform` or `Node3D` transforms).
  Godot's IK nodes (`IKModifier3D`, `TwoBoneIK3D`, Godot 4.6+) need a
  `Skeleton3D`, and the bear has none.
- **Overgrowth layers** (David Rosen, GDC 2014): a few key poses in
  code, a stride wheel driven by distance (not time), pass and reach
  poses, a tilt from acceleration, a bounce, and spring-damper
  blending. The movement code stays in control; the animation follows
  it ("First, do no harm to the gameplay"). Overgrowth's code is
  Apache-2.0 (`Source/Objects/riggedobject`). We may reuse it, but we
  must keep its licence notice.
- **Springs everywhere** (t3ssel8r, "Giving Personality to Procedural
  Animations using Math"): a second-order spring with three values:
  frequency f, damping zeta, response r. Use it for the root bob, head
  lag, ears, hump, limb follow and squash. Write it from the maths.
- **Verlet points** (Jakobsen, "Advanced Character Physics", GDC 2001;
  Rain World): points joined at fixed distances. Use them for a
  half-torn limb on a thread, a loose ear, and the tumble.
- **Aim IK, not knee IK.** A classic jointed teddy has no elbows or
  knees. Each limb is one stuffed piece on a disc joint. So the IK
  turns the limb at the shoulder or hip toward the target, with a
  small stretch or squash (about 10%). A soft bend can come from the
  vertex shader.

## The gait table (from the limbs the bear still has)

| Legs | 2 arms | 1 arm | 0 arms |
|---|---|---|---|
| 2 | walk, jog, jump; full climb | walk; the stump wobbles; limited climb | walk; no wall climb |
| 1 | hop (more bounce); arms help balance | hop, slower | hop, slow; small jumps only |
| 0 | arm-crawl, belly drags | one-arm drag: pull, rest, roll a little | wriggle and roll, very slow |

Each cell is data: gait id, speed scale, step length, phase offset for
each limb, body height, bounce, "can jump," and a climb profile.

Starting numbers for the climb profile (tune them):
- Maximum wall slope: 2 arms = the current maximum; 1 arm = about 65
  to 70 degrees; 0 arms = only the walkable limit.
- Stamina cost: 1 arm x1.6; 0 legs x1.3; wet x1.25.
- Grip: surface grip x paw state x (1 - 0.3 x wetness).
- Lost stuffing makes a limb weaker: more stamina cost, smaller jumps.

## Surface reactions

- **Feet:** one down-ray for each supporting limb from its hip or
  shoulder point, and one forward ray for the slope ahead. A planted
  foot stays fixed in the world. A foot steps when the stride wheel
  says so, or when it is too far from its home point. It moves on an
  eased arc.
- **Lean:** tilt the body toward the average ground normal, through a
  spring.
- **Slipping:** on low grip, the planted foot slides with the body by
  (1 - grip) each frame, with a little jitter and faster "scrabble"
  steps.
- **Sand:** lower the foot target by (sink depth x softness), lift the
  steps higher, and slow the gait.
- **Climbing hands:** each hand casts a fan of 3 to 5 short rays near
  the point where it wants to reach (above and to the side, in the
  climb direction). Score each hit by its distance to that point, how
  wall-like its normal is, and its grip. A hand moves when it is past
  its reach. The hands move in turn, like feet. No climbing points are
  necessary.

## The plush feel

- **Stuffing (0 to 1 for each part):** less stuffing means a softer,
  slower spring, more droop, and a slightly smaller part (re-mesh the
  SDF on a worker thread).
- **Wetness (0 to 1):** about x(1 + 1.5 x wetness) mass. Springs feel
  heavier and slower. Lower jumps, more stamina cost, more squash.
- Do not run a real soft body each frame. A spring-driven scale on
  each part fakes the squash.

## The look (one shader for all parts)

- Fibre-noise texture (128 x 128), wrap diffuse lighting, and a sheen
  term: Godot's Rim (about 0.6, tint about 0.5) or a "Charlie" sheen
  in a custom `light()` (Filament cloth model, Apache-2.0).
- Subsurface scattering is Forward+ only. Use wrap diffuse or
  BACKLIGHT on Mobile.
- A wear mask from the SDF convexity: the nose, the paws and the belly
  wear first and show the backing fabric.
- Stitches: dashes along the seam ring, `fract(u * stitch_count)`.
  They show gaps as the seam health falls.
- Wetness (Lagarde): darker albedo, lower roughness, less sheen and
  rim, flatter fibres. The shine dries before the colour.
- No alpha fringes and no shell fur: `discard` costs on integrated
  GPUs.
- Use `instance uniform` for wetness, damage and stuffing, so all
  parts share one material [unverified: check that it works in our
  Mobile build].

## Tearing (later, S-114)

Each joint has a seam health from 0 to 1:
1. Intact (above 0.7).
2. Frayed (0.4 to 0.7): stitch gaps, the joint spring x0.7, puffs of
   stuffing on hits.
3. Torn (0.1 to 0.4): looser joint, more droop, stuffing leaks.
4. Dangling (below 0.1): the limb hangs on a Verlet thread of 3 to 5
   points. It counts as missing for the gait.
5. Detached: the thread breaks. The limb becomes a Jolt rigid body,
   sleeps, and then becomes static, so it stays in the world for free.

Sewing back (later): the bear brings the limb to the socket, a short
stitch animation plays, and new thread in a different colour shows.

## Update order for each frame

1. Controller (as now): input, state machine, root, velocity, climb.
2. Limb set and gait: only when damage changes.
3. Surface probes: 4 to 8 ray casts.
4. Stride wheel: advance by the distance travelled.
5. Step planner for each limb: plant, swing or hold; slip from grip.
6. Body pose: key poses, tilt, lean, bounce, head look.
7. Springs.
8. IK for each limb.
9. Verlet: threads, dangling limbs, tumble.
10. Write the part transforms and the shader values.
11. Events: footstep dust, stuffing puffs, seam damage.

The full animation is a few tens of microseconds. Do not thread it.
Use worker threads only to re-mesh parts after damage.

## Build order (17 steps)

S-120 does steps 1 to 7. S-122 does steps 8 to 10. S-114 does steps
11 to 13 and 16.

1. Pose the parts from Rust; the bear stands in its rest pose.
2. The `SecondOrder` spring, with unit tests; root bob and head lag.
3. Turn toward the motion, idle pose, tilt from acceleration.
4. Stride wheel, pass and reach poses for two legs; walk and jog.
5. Foot probes, planted feet, swing arc, aim IK on the legs.
6. Slope lean, sinking, slipping from grip.
7. Arm swing, paw IK for idle and walk, ear and hump springs, squash
   on landing.
8. Climbing hands: fan rays, hands in turn, the body hangs.
9. `LimbSet` and gait table, debug keys to remove limbs, hop.
10. Arm-crawl, one-arm drag, wriggle; climb limits from the limbs.
11. Verlet: a thread and a two-point limb; dangling limbs.
12. Detach to a Jolt body, settle, become static; pick up and sew.
13. Seam health, damage, stuffing particles, floppiness, re-mesh.
14. Bear shader v1 (S-119 has a first version).
15. Shader v2: wear mask, stitches from seam health, wetness linked to
    mass and stamina.
16. Tumble: Verlet fake ragdoll, recovery by a pull at the chest.
17. Profile on an Intel UHD laptop.

## Sources (study in this order)

| Source | Use | Licence |
|---|---|---|
| Rosen, GDC 2014, https://www.gdcvault.com/play/1020583/Animation-Bootcamp-An-Indie-Approach (video https://youtube.com/watch?v=LNidsMesxSE) | The core pose system | Talk |
| Overgrowth source, https://github.com/WolfireGames/overgrowth (`Source/Objects/riggedobject`) | Stride wheel, springs, layer order | Apache-2.0 (keep notice) |
| t3ssel8r, https://www.youtube.com/watch?v=KPoeNZZ6H4s | The spring type | Video; write own code |
| Jakobsen, Advanced Character Physics, https://www.cs.cmu.edu/afs/cs/academic/class/15462-s13/www/lec_slides/Jakobsen.pdf | Verlet threads and fake ragdoll | Paper |
| WeaverDev gecko tutorial, https://weaverdev.io/projects/proc-anim-tutorial/ | Step triggers, foot planting | Tutorial; check code licence |
| Rain World, GDC 2016, https://www.gdcvault.com/play/1023475/Animation-Bootcamp-Rainworld-Animation | Points at fixed distances | Talk |
| Hecker et al. 2008 (Spore), https://www.chrishecker.com/Real-time_Motion_Retargeting_to_Highly_Varied_User-Created_Morphologies | Limbs by role and side | Paper |
| Godot spatial shader reference (RIM, BACKLIGHT, `light()`) | The plush shader | Docs |
| Filament cloth model, https://google.github.io/filament/Filament.md.html | Sheen, wrap diffuse | Apache-2.0 (verify) |
| Lagarde, wet surfaces, https://seblagarde.wordpress.com/2013/04/14/water-drop-3b-physically-based-wet-surfaces/ | Wet fur | Blog; ideas only |
| Making of Shadow of the Colossus (2005), https://lukasz.dk/files/making_of_sotc.pdf | IK on top of few motions, on weak hardware | Interview |

## What S-122 found and built (2026-11-14)

A search for open sources found nothing that covers limb loss or
hopping gaits. These sources are near:
- **Gibbon: Beyond the Trees** (Hemisphere Games, 2022,
  https://hemispheregames.com/2022/03/19/physics-based-animation-in-gibbon-beyond-the-trees):
  a planner picks the next hold, and the body hangs from the hand.
  The rig is solved in layers (centre of mass first, then 5 points,
  then the skeleton). Article only; no code licence.
- **Rune Johansen, procedural creatures**
  (https://blog.runevision.com/2025/01/procedural-creature-progress-2021-2024.html):
  ray casts find the footsteps; no missing legs. No code.
- **Overgrowth source** (Apache-2.0): the repo has no climbing code
  that the search found.

What S-122 built from this doc, with our own code:
- A hold for each hand and foot. A step starts when the body is 5.5 cm
  from the home of the limb. The fan has 4 rays for a hand and 2 for
  a foot. The score is `facing + grip - 0.5 x distance / spread`.
- The partner limb (the other hand, the other foot) must hold before a
  limb steps. This gives the steps in turn with no timer.
- The gait contacts are general: the feet in a walk and a hop, the
  paws in a crawl and a drag. The same step planner moves all of them.

