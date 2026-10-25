# State

Updated: 2026-10-25 (S-009 Cleanup done)

Rewrite this file, don't add to it. Keep it under 100 lines.

## Phase

**Gate A passed** (2026-10-25). The first playable
(`prototypes/03-first-playable.md`, B1 to B11) is built and live, as a
sketch (0021). Mikey wants a better-built slice of the first version,
with real mini games (0043) that any friend can play, its body
changing how (0044). Those stand on physics, so the physics test
(`prototypes/04-physics-test.md`) came first; it is built, and Mikey's
first impression is good. His first playtest notes (B12) reshape the
tent: a bigger play room, upright surgery, grooming on the starting
screen, and many more body plans. The first version is a dev release with
everything open (0045).

## Agenda

How it works: `kit/RULES.md` section 1. Mikey starts a session by
saying "Run S-NNN" in a new chat of the kind, model and effort shown.
A session ends by deleting its row, flipping rows it unblocks to
**ready**, and adding follow-ups at the bottom. Next number: S-013.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-006 | **Red team:** the physics test's result, 0043, 0044 and the body-ability framework, before any mini game is built on them. A fresh chat that didn't build it. | Claude Code | Opus 5.5, high | waiting on S-008 |
| S-007 | **Build:** the skateboard grind course, the first real mini game. | Claude Code | Opus 5.5, high | waiting on S-006, S-012 |
| S-008 | **Brain** with Mikey: which mini games, what each gives, where they happen; friend looks (fur, head shapes); which body plans first, in what order, and how each moves (0045, `systems/friend-body`); where the play room is (Q-036). | Project | Opus 5.5, medium | **ready** |
| S-012 | **Build:** Mikey's first playtest notes (`prototypes/03-first-playable.md`, Result): a bigger play room where several friends wander, with a camera that zooms in and out and a stress test for how many friends the phone holds (`systems/play`; where the room is, Q-036: guess and list it); surgery upright and zoomed in; the starting screen split into the entrance and the grooming screen (`systems/ui-ux`); a dev menu to reach every mini game and test friend (0045); sewing a missing part's hole shut, an open hole rotting, and the physics test's neck snag (fix it; a very hard neck hit pulls the neck seam, which takes more pulls than a limb; V-020). Skip patches and new body plans. | Claude Code | Opus 5.5, high | **ready** |
| S-011 | **Verifier** (needs web access to docs sites, Wikipedia, Defold and Esoteric pages, which this container blocked): V-007, V-013 (Nightmare Before Christmas half), V-017 (references); V-018, V-019 need a minimal repro on Godot 4.7.1, so pair them with a Build session. | Claude Code (with web) | Sonnet 5.5, medium | **ready** |

**Order** (Mikey asked, 2026-10-25):
1. S-009 (Cleanup) is done. (S-005, Mikey playing the physics test,
   is done: his impressions are in.)
2. Then in tandem: S-012 (game code), S-008 (Project chat, plan docs),
   S-011 (`verify.md` and the docs it checks).
3. S-006 once S-008 is done.
4. S-007 last: after S-006 and S-012 (both change game code).

## Blockers

None.

## Where to look

- Decisions, one line each: `decisions/README.md`.
- Open questions and claims to check: `open-questions.md`, `verify.md`.
  `check.py` prints totals.
