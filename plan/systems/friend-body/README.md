---
status: idea
last_reviewed: 2026-10-23
depends_on: [species-habitats, condition]
feeds: [care-touch, condition, needs-growth, expeditions, art-direction]
---
# Friend body

## In charge of

The physical friend: its parts, the seams that hold them together,
its skeleton, and how it moves, flops and comes apart.

## Not in charge of

- What state the fabric and stuffing are in: `condition`.
- The gestures the player uses to tear or sew: `care-touch`.
- How heavy or thin it is from eating: `needs-growth` decides,
  this system shows it.
- How it's drawn: `art-direction`.

## Seed notes (from Mikey's first ideas, unconfirmed)

- Limbs can be pulled off and sewn back on.
- Gets visibly big when overfed, skinny when underfed.
- Stuffed, stitched friends (teddy bear, the dolls from *9*).
- Squishiness will likely need to be faked with springs or a wobble
  effect [unverified].

- Insides (recording 1, 2026-10-24): mostly fluff; maybe robotic
  bones or hydraulic ligaments full of colored liquid that spills when
  a part comes off; a heart or battery in the chest (stamina, strength,
  mood); a chip brain in the head (0026); a voice box, so a friend that
  can't speak needs surgery. Insides follow what is fun to cut, pull
  and spill.
- Build shows in motion (recording 2, 2026-10-24): thin friends wobble
  as they walk, fat ones bounce; thin long ears droop and swing, thick
  ones answer differently; dry joints squeak until oiled.
- Quality varies: better limbs, buttons, capes and accessories change
  what a friend suits (recording 1).
- Body plans (Mikey's playtest, 2026-10-25): two legs, four legs,
  insect legs, tentacles, a plush ghost-like friend that gently floats
  without legs (0037: never an actual ghost), legless oozing friends
  like Slime Rancher's. As many as possible in the dev release
  (proposed 0045).
- A missing part's hole can be sewn shut; a friend can live without
  that leg. A hole left open leaks and starts to rot (Mikey).
- Skin can be ripped off and patched with a random patch of cloth
  (Mikey). Who owns what (Mikey: go with what an experienced developer
  would pick; Claude's pick, Q-039): this system stores the skin on
  each part (where it's torn, where it's patched); `condition` decides
  what an open tear does over time (leak, rot, like an open hole);
  `art-direction` generates and draws the patches. The patchwork
  puzzle mini game is one way to make a patch (`play`).

## First questions

- What is a "part", and what is a "seam"?
- What does a friend with a missing limb do, beyond hopping (0025)?
- Can parts be swapped between friends?
