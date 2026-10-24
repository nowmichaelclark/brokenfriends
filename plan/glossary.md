# Glossary

One meaning per word. Use these exactly; add new terms in the session
that introduces them. Alphabetical within each section.

## Game terms

Working definitions from the first pitch session (2026-10-23). None
are owned by a system yet; Phase 2 assigns owners.

Format: **Term**: definition in one or two sentences. *Owned by:*
system. *Not to be confused with:* other term, if any.

Still to define: species, seam, stuffing, fluff, condition, wetness,
mold, habitat, expedition, package, theme.

**Camp**: the place the player mostly stays, with their friends,
tools and tents. A friend that is not on a quest is at camp. *Owned
by:* no system yet. *Not to be confused with:* habitat, expedition.

**Donation**: moving a part from one friend to another: a rotting
limb replaced, or a healthy spare part given. The donor is left without
the part (0025). A sewn part is permanent (0011). *Owned by:* no system yet. *Not to be confused with:*
repair.

**Friend**: a stitched, doll-like being the player cares for, plays
with and repairs; the game's word for them (decision 0022). Not
"creature". *Owned by:* no system yet. *Not to be confused with:*
species.

**Keepsake**: a part left behind when a friend dies. Died at camp:
nearly all its parts; died on a quest: one or two (decision 0026).
The friend itself never comes back (0011). *Owned by:* no system yet.

**Part**: a piece of a friend: a limb, an eye, an accessory. A part
can rot, be ripped off, be sewn on, be added, or be donated to another
friend. *Owned by:* no system yet.

**Play**: interacting with a friend for its own sake: poking,
feeding, minigames, watching it react. The preferred word over
"train". *Owned by:* no system yet.

**Quest**: a task a friend is sent on and comes back from changed.
*Owned by:* no system yet. *Not to be confused with:* minigame.

**Repair**: ripping off a rotting part and sewing on a replacement or
an addition. A repaired friend is never the same as before. *Owned
by:* no system yet. *Not to be confused with:* donation.

## Planning terms

**Decision**: a choice Mikey has made, recorded in `decisions/` and
signed with his name and the date. Proposals are not decisions.

**Gate**: a strong-model review at the end of a phase that says go or
no-go. See `kit/PHASES.md`.

**Locked**: a doc status meaning Mikey approved it and build plans may
rely on it. Only Mikey sets it.

**Pack**: the single block of text a session starts from, made by
`kit/tools/pack.py`: brief, rules, task and the needed files.

**Pillar**: a short statement every feature is tested against. See
`00-vision/pillars.md`.

**Prototype**: a small, throwaway build that answers one question by
playing it. See `prototypes/`.

**Red team**: a session whose only job is to find problems in docs.

**Session**: one conversation with one model, doing one job, from one
brief in `kit/sessions/`.

**Spike**: a tiny test build that answers a technical question (does X
work on Y?), as opposed to a prototype, which answers a design
question.

**Tier**: how strong and costly a model is: F (free or cheap), M (mid),
S (strong). See `kit/PHASES.md`.

**Unverified**: a technical claim not yet checked against a real
source, tagged `[unverified]` and listed in `verify.md`.
