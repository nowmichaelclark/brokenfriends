# State

Updated: 2026-10-25 (Cleanup done; the red team split into a loop)

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
**ready**, and adding follow-ups at the bottom. Next number: S-016.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-006 | **Red team 1, physics:** the physics test's result (`prototypes/04-physics-test.md`), tearing and the neck (0025, 0026), and 0044's body-ability framework. Review file only. Sonnet, because Opus built it: a different model catches different things. | Claude Code | Sonnet 5.5, high | **ready** |
| S-007 | **Build:** the skateboard grind course, the first real mini game. | Claude Code | Opus 5.5, high | waiting on S-012, S-015 |
| S-008 | **Brain** with Mikey: which mini games, what each gives, where they happen; friend looks (fur, head shapes); which body plans first, in what order, and how each moves (0045, `systems/friend-body`); where the play room is (Q-036). | Project | Opus 5.5, medium | **ready** |
| S-012 | **Build:** Mikey's first playtest notes (`prototypes/03-first-playable.md`, Result): a bigger play room where several friends wander, with a camera that zooms in and out and a stress test for how many friends the phone holds (`systems/play`; where the room is, Q-036: guess and list it); surgery upright and zoomed in; the starting screen split into the entrance and the grooming screen (`systems/ui-ux`); a dev menu to reach every mini game and test friend (0045); sewing a missing part's hole shut, an open hole rotting, and the physics test's neck snag (fix it; a very hard neck hit pulls the neck seam, which takes more pulls than a limb; V-020). Skip patches and new body plans. | Claude Code | Opus 5.5, high | **ready** |
| S-011 | **Verifier** (needs web access to docs sites, Wikipedia, Defold and Esoteric pages, which this container blocked): V-007, V-013 (Nightmare Before Christmas half), V-017 (references); V-018, V-019 need a minimal repro on Godot 4.7.1, so pair them with a Build session. | Claude Code (with web) | Sonnet 5.5, medium | **ready** |
| S-013 | **Red team 2, plans:** 0043, 0045 and what S-008 decides (mini games, body plans, the play room). Review file only. | Claude Code | Sonnet 5.5, high | waiting on S-008 |
| S-014 | **Brain** with Mikey: go through both red team reviews, finding by finding; decide what to fix, what to drop, what to test. | Project | Opus 5.5, medium | waiting on S-006, S-013 |
| S-015 | **Build:** fix what S-014 chose, run the playthrough and physics tests, play it, change it, test again. If a fix is big or shaky, add another red team row for it before S-007. | Claude Code | Opus 5.5, high | waiting on S-014, S-012 |

**Order** (Mikey asked, 2026-10-25):
1. S-009 (Cleanup) is done. (S-005, Mikey playing the physics test,
   is done: his impressions are in.)
2. Now, in tandem: S-012 (game code), S-008 (Project chat, plan docs),
   S-011 (`verify.md`), S-006 (a review file only).
3. S-013 once S-008 is done.
4. The loop: S-014 (decide with Mikey), then S-015 (fix, test, fix).
5. S-007 last, on what the loop left standing.

## Blockers

None.

## Where to look

- Decisions, one line each: `decisions/README.md`.
- Open questions and claims to check: `open-questions.md`, `verify.md`.
  `check.py` prints totals.
