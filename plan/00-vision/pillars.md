---
status: draft
---
# Pillars

Five short statements every feature is tested against. Hold a feature
up and answer yes or no. Settled with Mikey on 2026-10-24; the rules
behind them are in decisions 0011 and 0012.

A good pillar rejects something real. Each one below names a feature it
would turn down.

## 1. The body is the interface

**The pillar:** everything the player must act on is visible on the
creature's body, not behind a menu.

**What it means in practice:** rot, mold, mud, bugs, hunger and a badly
sewn seam all show on the creature itself. Touching it produces a
visible, physical reaction. The creature is the primary interface;
nothing the player must act on is hidden behind a screen. A redundant
non-menu cue (a wobble, an outline) is allowed, for players who can't
read the body alone.

**A feature it would reject:** a "condition: 62%" inspect panel, or a
health bar floating over a sick creature.

## 2. The friend is never the same as before

**The pillar:** every repair, addition, donation or transplant leaves a
permanent mark, and death is final.

**What it means in practice:** parts persist and can move between
friends. A rotting limb is replaced; a healthy spare leg is given to a
friend who has one; a dead friend's button eye is sewn onto a living
one. What is refused is going back: no restore, no undo, no revive. A
creature must never be able to reassemble itself into the one the
player met.

**A feature it would reject:** a "restore original parts" button, a
revive item, or a quest reward that returns a creature to how it was
found.

## 3. Every loss has a cause the player could have seen

**The pillar:** nothing dies offscreen without a visible history.

**What it means in practice:** rot needs a source: a wound, mud, bugs,
an unclean camp. Dirt speeds rot but never starts it. A death traces
back to a wound, a dirty camp, or a quest the player chose. Absence
alone never kills a healthy creature.

**A feature it would reject:** a random "your creature caught mold
while you were away" event, or death by old age.

## 4. Nothing holds the player

**The pillar:** the game never pressures the player to return, and
absence is never itself the cause of a loss.

**What it means in practice:** no gates, no energy, no streaks, no
expiring events. A two-minute visit and a two-hour session are both
complete experiences. Time away can change a creature (pillar 3), but
it never costs a bonus, a reward or a guilt screen.

**A feature it would reject:** an energy timer, a daily login streak, a
limited-time event, or a "you were away 6 days" message.

## 5. The gross is always in the player's hands

**The pillar:** every gross thing is something the player does, or can
reach and fix; never something done to a creature they can't help.

**What it means in practice:** ripping, sewing, cleaning and treating
are the player's acts. Rot on a creature the player can see and reach
is fair game; rot on a creature the player cannot reach or treat is
not. The tone is wholesomely gross, never grim.

**A feature it would reject:** a screaming pain face, blood and gore
for shock, or rot crossing a camp the player has no way to clean.

## Why 2 and 3 are separate

Both carry "accept death", so a later session may try to merge them.
Don't. Pillar 2 is about **undo**: a creature that died or changed can
never be returned. Pillar 3 is about **cause**: why it changed or died
is always something the player could have seen. A feature can pass one
and fail the other.

## Not pillars

- The tone fence (blood, gore, revive, story) lives in
  `00-vision/non-goals.md` (decision 0012).
- Parked features, not tests: donation rules, keepsakes, camp tools,
  the open world, multiple camps. See `open-questions.md`.
