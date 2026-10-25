---
status: planned
date: 2026-10-25
tests: [friend-body, play, platforms-tech]
---
# Prototype 4: friends as soft ragdolls

## The question

Are friends fun to push, fling and watch fall over as soft ragdolls
(head, body, arms and legs joined by springy seams), and does that run
smoothly on Mikey's Moto G Power 2022 and in its browser? Every mini
game will stand on this (`inbox/2026-10-25-friend-looks-and-minigames.md`,
proposed 0043 and 0044), so it comes first.

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

Not built yet.

## What changed in the docs

None yet.
