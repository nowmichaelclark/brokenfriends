---
status: building
date: 2026-10-25
tests: [friend-body, play, platforms-tech]
---
# Prototype 4: friends as soft ragdolls

## The question

Are friends fun to push, fling and watch fall over as soft ragdolls
(head, body, arms and legs joined by springy seams), and does that run
smoothly on Mikey's Moto G Power 2022 and in its browser? Every mini
game will stand on this (the 2026-10-25 looks and mini games note, digested; see git history,
0043 and 0044), so it comes first.

## Smallest build

A separate scene, reached by a debug flag (`#physics`), not from the
tent:

- **One friend as a ragdoll.** Each part is its own body, joined at
  the seam by a springy joint, using the friend's real generated look
  and parts. Missing parts are simply missing. Godot's built-in 2D
  physics (decision 0005).
- **The body sets the physics.** Stuffing sets weight and bounce;
  wetness adds weight; a loose seam is a weak joint that can tear on a
  hard landing, and the torn part stays torn in the friend's data.
- **Drag and fling** the friend with a finger.
- **A ramp, a step and a skateboard** to land on, ride down and fall
  off. A friend with no legs sits on the board.
- **Debug buttons** (debug only, so text is fine): next friend (tiny,
  huge, no legs, one arm, soaked), and "three friends at once".
- **Speed:** the `#perf` overlay with physics time added.

## Out of scope

Any real mini game, scores, prizes, skills, the player steering the
friend, the half pipe, the friend keeping its balance by itself
("active ragdoll"), sound beyond the existing voice squeaks, and
linking the scene into the tent or map.

## How Mikey will judge it

Ten minutes on the phone and in the browser:
- **Fun:** do you keep flinging them because it's funny? Does it feel
  like a plush toy (soft, bouncy), not a stiff puppet or jelly?
- **Bodies:** does a tiny friend feel different from a huge one, a
  soaked one from a dry one, a legless one on the board from a
  two-legged one?
- **Tearing:** when an arm tears off on a hard landing, is it funny
  and fair, or annoying?
- **Speed:** smooth with three friends at once?

Yes: build the skateboard grind course on it next. Not sure: tune the
springs and weights, then play again. No: rethink (scripted motion
instead of physics, or the Rapier plugin, decision 0005).

## Done when (for the build)

- The playthrough test still ends `FLOW OK`, and a new physics test
  shows a friend dropped from a height lands, settles, and stays in one
  piece; a friend with a weak seam dropped hard loses that part.
- Frame time and draw calls at 4x slowdown in `#perf` are listed
  below for one friend and for three.
- Web and Android builds pass on GitHub.
- `game/README.md` lists the guesses (spring strengths, weights,
  tear force) under "Assumed in this build".

## Result

**Built; first impressions in** (Mikey, 2026-10-25, S-004): "great.
It works pretty well." Once the skateboard got caught in a friend's
neck; he'd expect "the head to come off... or for it to not get caught
in the neck". He chose: fix the snag, and let a very hard hit on the
neck count as a pull on its seam, with the neck taking more pulls than
a limb (0025), so heads come off rarely and only after a hard hit. The
snag is likely the board overlapping the gap at the joint [unverified,
V-020]. Full ten minutes still to come (S-005). Open `#physics` (`&three` for three friends, `&perf`
for speed); `game/README.md` says what it does and lists every guess
(weights, bounce, swing, what counts as a hard landing).

What building it showed:
- A drop from the top of the screen lands at about 1,700 pixels a
  second; a hard drop at about 3,000. A part hitting something at 2,500
  or more counts as a hard pull (0025), so a drop never tears a healthy
  friend, and a loose seam dropped hard loses its part.
- Two Godot settings broke the ragdoll and are off: the pin joint's own
  angular limits wrenched seams far open on landing [unverified,
  V-018], and continuous collision detection made friends bounce
  higher than they fell [unverified, V-019]. Each part's swing is
  limited by a spring in our code instead.
- Godot's joints have no breaking force; tearing is scripted, as V-010
  expected (the engine's class list for PinJoint2D shows none).

The physics test in the playthrough (`FLOW OK`): every test friend
dropped from a height lands, settles and stays whole; a loose seam
dropped hard loses that part and the others hold; a finger picks a
friend up and flings it; the board rolls down the ramp with a legless
friend on it; three at once land and hold together.

Speed at 4x slowdown, in the test browser at the Moto's pixel density
(the test machine draws in software, so only good for comparing):

| | Frame time (avg) | Physics | Draw calls |
|---|---|---|---|
| Tent, before this session | 62–80 ms | (not shown) | 39 |
| Tent, after (split code) | 65–85 ms | 1–2 ms | 39 |
| Physics test, one friend | 43–54 ms | 4–8 ms | 43 |
| Physics test, three friends | 45–56 ms | 4–10 ms | 55 |

Each a range over 20 seconds from the start, so the drop, landing and
settling; music off (no extension in the local build). Physics stays
small next to drawing; the real test is Mikey's phone.

## What changed in the docs

- `game/README.md`: the physics test and its guesses.
- `glossary.md`: ragdoll, seam.
- `verify.md`: V-018, V-019.
