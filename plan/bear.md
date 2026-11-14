# The bear: model, materials and animation

Status: draft, 2026-11-14, from Mikey's notes and reference photos.
Decisions R17 and R18 are Mikey's. The method is a proposal.

## The look (R17)

The bear looks like an old, classic teddy bear (early 1900s style):
- a long body with a small hump at the back, and long arms that reach
  past the hips
- long, thin legs with big feet
- a long snout, a stitched black nose and mouth, small black button
  eyes, round ears
- pale felt pads on the paws and the feet
- short, worn mohair fur in honey or tan colours
- simple shapes, with joints at the neck, the shoulders and the hips

## How the bear is made: in code, not from a file

Rust makes the bear when the game starts. No model file is necessary.

1. Each part (head, snout, ears, body, two arms, two legs, paw pads) is
   an SDF made from simple shapes: ellipsoids, capsules and a smooth
   union. The seed or a "bear recipe" sets the sizes, so later friends
   can have different shapes.
2. `fast-surface-nets` (MIT/Apache) changes each SDF into a low-poly
   mesh. Each part is a separate mesh, so a part can come off.
3. Each part has a seam ring where it joins the body. The seam is
   where it tears, and where stuffing comes out.

We do not use a downloaded model, for these reasons:
- Most free models have a "royalty free" licence. That kind of
  licence usually does not let us put the model file in a public
  repo.
- A downloaded model is one piece, not separate parts with seams.
- Procedural animation does not need an artist's skeleton.

## Materials

- **Fur:** a small texture made in code (short, uneven fibres) and a
  soft rim light on the edges. The silhouette gets a little noise, so
  it looks fuzzy. No shell fur or hair cards; they cost too much.
- **Wet fur:** one value from 0 to 1 for each part. Wet fur is darker,
  flatter and a little shiny. Later, wet parts can also be heavier.
- **Stuffing:** small white particles that fall out of a torn seam. A
  part with less stuffing becomes thinner and floppier.
- **Plush bounce:** each part has a small spring that squashes and
  stretches it when it hits something. It is cheap: a few numbers for
  each part, not a soft-body simulation.

## Procedural animation (R18)

The method comes from the research in `research/bear-animation.md`.
In short:

- **No skeleton and no animation clips.** Rust poses each part mesh
  directly.
- **Overgrowth layers** (David Rosen, GDC 2014): a few key poses in
  code, a stride wheel driven by distance, a tilt from acceleration, a
  bounce, and spring blending. The movement code stays in control.
- **Springs everywhere:** the body, the head, the ears, the hump and
  the limbs follow their targets through springs, so the bear is soft
  and floppy.
- **Aim IK:** a classic jointed teddy has no elbows or knees. Each limb
  turns at its disc joint toward a target, with a small stretch.
- **Feet and paws find the surface** with short rays. Planted feet
  stay fixed, slip on low grip and sink in sand. Climbing hands choose
  the best hold from a fan of rays, so no climbing points are
  necessary.
- **The gait comes from the limbs,** from a table: walk, hop, arm-crawl,
  one-arm drag, or wriggle. The same table gives the climb limits.
- **Verlet points** (Rain World) for a half-torn limb on a thread and
  for the tumble.

*Shadow of the Colossus* is a feel reference only. Its climbing
methods are not published.

## Order of work

1. S-119 (done): the bear model in code, with separate parts, fur and
   the wet value.
2. S-120 (done): animation part 1, the body and the walk
   (`rust/game/src/anim.rs`, `spring.rs`).
3. S-122 (done): animation part 2, climbing hands and limb sets
   (`anim.rs`, `limbs.rs`).
4. S-114 (done): tumbling (a Jolt ragdoll), seams that tear, dangling
   limbs on a thread, lost limbs that stay in the world, and sewing.
