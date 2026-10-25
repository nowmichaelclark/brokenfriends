# Glossary

One meaning per word. Use these exactly; add new terms in the session
that introduces them. Alphabetical within each section.

## Game terms

Working definitions from the first pitch session (2026-10-23). None
are owned by a system yet; Phase 2 assigns owners.

Format: **Term**: definition in one or two sentences. *Owned by:*
system. *Not to be confused with:* other term, if any.

Still to define: species, stuffing, fluff, condition, wetness,
mold, habitat, expedition, package, theme.

**Body plan**: the overall shape a friend is built on, which sets how
it moves: two legs, four legs, insect legs, tentacles, floating,
oozing (0045). *Owned by:* no system yet. *Not to be confused with:*
species.

**Camp**: the place the player mostly stays, with their friends,
tools and tents. A friend that is not on a quest is at camp. *Owned
by:* no system yet. *Not to be confused with:* habitat, expedition.

**Donation**: moving a part from one friend to another: a rotting
limb replaced, or a healthy spare part given. The donor is left without
the part (0025). A sewn part is permanent (0011). *Owned by:* no system yet. *Not to be confused with:*
repair.

**Entrance**: the first screen, showing a new friend or new items that
have arrived and offering what to do next (Mikey, 2026-10-25). *Owned
by:* ui-ux. *Not to be confused with:* grooming screen.

**Friend**: a stitched, doll-like being the player cares for, plays
with and repairs; the game's word for them (decision 0022). Not
"creature". *Owned by:* no system yet. *Not to be confused with:*
species.

**Grooming screen**: where a friend stands to be washed, dried,
brushed and given accessories; not surgery (Mikey, 2026-10-25).
*Owned by:* ui-ux. *Not to be confused with:* entrance, the surgery
table (0031).

**Keepsake**: a part left behind when a friend dies. Died at camp:
nearly all its parts; died on a quest: one or two (decision 0026).
The friend itself never comes back (0011). *Owned by:* no system yet.

**Part**: a piece of a friend: a limb, an eye, an accessory. A part
can rot, be ripped off, be sewn on, be added, or be donated to another
friend. *Owned by:* no system yet.

**Patch**: a piece of cloth sewn over a place where a friend's skin
was ripped off; random, so each friend looks its own (Mikey,
2026-10-25). *Owned by:* friend-body (drawn by art-direction). *Not
to be confused with:* repair.

**Play**: interacting with a friend for its own sake: poking,
feeding, minigames, watching it react. The preferred word over
"train". *Owned by:* no system yet.

**Quest**: a task a friend is sent on and comes back from changed.
*Owned by:* no system yet. *Not to be confused with:* minigame.

**Ragdoll**: a friend built as separate physics bodies (body, head,
limbs) joined at its seams, so it flops, falls and can be flung
(prototype 4). *Owned by:* no system yet.

**Repair**: ripping off a rotting part and sewing on a replacement or
an addition. A repaired friend is never the same as before. *Owned
by:* no system yet. *Not to be confused with:* donation.

**Seam**: where a part joins the body. Each hard pull loosens it;
enough, and the part comes off (0025). The physics test also counts a
hard landing as a pull (a guess, prototype 4). *Owned by:* no system
yet. *Not to be confused with:* stitches (the thread of a repair).

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
