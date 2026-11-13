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

The bear has no hand-made animations. The game makes each pose from
the body, the limbs that the bear still has, and the surface under
it. This is the method of *Overgrowth* (David Rosen, GDC 2014, "An
Indie Approach to Procedural Animation") and *Rain World*:

- **A few key poses,** made in code: stand, walk step, climb reach,
  crawl. The game blends between them.
- **Springs:** the body and the limbs follow their targets with
  springs, so the movement is soft and floppy, like a plush toy.
- **IK from ray casts:** each foot and each paw casts a ray to the
  ground or the wall. IK (inverse kinematics) bends the limb so that
  the paw touches the hit point. On rough rock, each paw finds its own
  place.
- **The gait comes from the limbs:**
  - two legs: walk
  - one leg: hop, slower, and the bear leans to one side
  - no legs: pull the body with the arms (crawl)
  - no arms and no legs: roll and wriggle
- **Climbing comes from the limbs:** each arm and leg adds grip and
  reach. With one arm, the bear climbs only less steep slopes and uses
  more stamina. With no arms, it cannot climb walls.
- **The surface changes the movement:** on sand, the steps are short
  and the feet sink a little. On a slippery slope, the paws slip, and
  the body leans into the wall.

*Shadow of the Colossus* is a good feel reference: the hero holds on,
swings and struggles in a way that reacts to the surface. Its exact
methods are not published in detail [unverified]. Thus, we use the
Overgrowth and Rain World methods, which are published.

## Order of work

1. S-119: the bear model in code, with separate parts, fur and the
   wet value.
2. S-120: procedural animation: key poses, springs, IK on the ground
   and the wall, and the gait from the limbs.
3. S-114: tumbling, tearing and lost limbs (uses 1 and 2).
