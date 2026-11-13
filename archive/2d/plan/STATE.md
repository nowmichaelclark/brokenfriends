# State

Updated: 2026-10-25 (S-012 built Mikey's first playtest notes)

Rewrite this file, don't add to it. Keep it under 100 lines.

## Phase

**Gate A passed** (2026-10-25). The first playable
(`prototypes/03-first-playable.md`, B1 to B11) is built and live, as a
sketch (0021). Mikey wants a better-built slice of the first version,
with real mini games (0043) that any friend can play, its body
changing how (0044). Those stand on physics, so the physics test
(`prototypes/04-physics-test.md`) came first; it is built, and Mikey's
first impression is good. His first playtest notes (B12) reshaped the
tent, and S-012 built them (B13): the entrance, the grooming screen
and an upright surgery table, a play room where friends wander
together, a dev menu, holes sewn shut or left to rot, and the neck
snag fixed. Many more body plans are still to come. The first version
is a dev release with everything open (0045).

## Agenda

How it works: `kit/RULES.md` section 1. Mikey starts a session by
saying "Run S-NNN" in a new chat of the kind, model and effort shown.
A session ends by deleting its row, flipping rows it unblocks to
**ready**, and adding follow-ups at the bottom. Next number: S-018.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-007 | **Build:** the skateboard grind course, the first real mini game. | Claude Code | Opus 5.5, high | waiting on S-015 |
| S-008 | **Brain** with Mikey: which mini games, what each gives, where they happen; friend looks (fur, head shapes); which body plans first, in what order, and how each moves (0045, `systems/friend-body`); where the play room is (Q-036). | Project | Opus 5.5, medium | **ready** |
| S-011 | **Verifier** (needs web access to docs sites, Wikipedia, Defold and Esoteric pages, which this container blocked): V-007, V-013 (Nightmare Before Christmas half), V-017 (references); V-018, V-019 need a minimal repro on Godot 4.7.1, so pair them with a Build session. | Claude Code (with web) | Sonnet 5.5, medium | **ready** |
| S-013 | **Red team 2, plans:** 0043, 0045 and what S-008 decides (mini games, body plans, the play room). Review file only. S-008 ends by making its pack (`kit/tools/pack.py red-team`). | DeepSeek chat (browser, free) | DeepSeek, thinking on | waiting on S-008 |
| S-014 | **Brain** with Mikey: save S-013's DeepSeek review into `reviews/` (Mikey pastes it in; S-006's is saved), then go through both finding by finding, skipping what S-012 already fixed; decide what to fix, what to drop, what to test. | Project | Opus 5.5, medium | waiting on S-013 |
| S-015 | **Build:** fix what S-014 chose, run the playthrough and physics tests, play it, change it, test again. If a fix is big or shaky, add another red team row for it before S-007. | Claude Code | Opus 5.5, high | waiting on S-014 |
| S-016 | **Play:** Mikey plays S-012's build on the phone: the entrance, grooming and the upright table, sewing a hole shut, the play room (pinch to zoom). The stress test: `#crowd=10&perf`, then 20 and 40, noting where it stops feeling smooth. Answer the guesses in `game/README.md` ("Assumed"), above all where feeding and mold spray belong and where the play room is (Q-036). Notes go to `prototypes/03-first-playable.md`, Result. | Phone, then a Project chat | Mikey; Opus 5.5, low for the notes | **ready** |
| S-017 | **Cleanup** (check.py asks for one): clear ticked `verify.md` items (V-020 and the moot ones), the log down to five, anything check.py reports. | Claude Code | Sonnet 5.5, medium | **ready** |

**Order** (Mikey asked, 2026-10-25):
1. S-009 (Cleanup) is done. (S-005, Mikey playing the physics test,
   is done: his impressions are in.)
2. Now, in tandem: S-016 (Mikey plays S-012's build), S-008 (Project
   chat, plan docs), S-011 (`verify.md`), S-017 (Cleanup). S-012 is done, with the
   DeepSeek review's findings 1, 3, 7, 10, 12 and 14.
3. S-013 once S-008 is done.
4. The loop: S-014 (decide with Mikey), then S-015 (fix, test, fix).
5. S-007 last, on what the loop left standing.

## Blockers

None.

## Where to look

- Decisions, one line each: `decisions/README.md`.
- Open questions and claims to check: `open-questions.md`, `verify.md`.
  `check.py` prints totals.
