# State

Updated: 2026-10-25 (S-004: 0043, 0044 signed; first playtest notes in)

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
screen, and many more body plans.

## Agenda

How it works: `kit/RULES.md` section 1. Mikey starts a session by
saying "Run S-NNN" in a new chat of the kind, model and effort shown.
A session ends by deleting its row, flipping rows it unblocks to
**ready**, and adding follow-ups at the bottom. Next number: S-013.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-005 | **Mikey plays** the physics test on his phone and in the browser (`#physics`, `#physics&three`), judging by `prototypes/04-physics-test.md`. Live on the web build. | (phone) | (no model) | **ready** |
| S-006 | **Red team:** the physics test's result, 0043, 0044 and the body-ability framework, before any mini game is built on them. A fresh chat that didn't build it. | Claude Code | Fable 5.1, high (or Opus 5.5) | waiting on S-005 |
| S-007 | **Build:** the skateboard grind course, the first real mini game. | Claude Code | Opus 5.5, high | waiting on S-006 |
| S-008 | **Brain** with Mikey: which mini games, what each gives, where they happen; friend looks (fur, head shapes) and body plans (Q-035); limbs as tools (Q-038); a head lost mid-game (Q-034). | Project | Opus 5.5, medium | **ready** |
| S-009 | **Cleanup:** the regular one (RULES section 1). | Claude Code | Sonnet 5.5, medium | waiting until `check.py` says one is due |
| S-012 | **Build:** Mikey's first playtest notes (`prototypes/03-first-playable.md`, Result): a bigger play room with several friends (Q-036; list the guesses), surgery upright and zoomed in, the starting screen for grooming only, sewing a missing part's hole shut, an open hole rotting, and the physics test's neck snag (fix it; a very hard neck hit pulls the neck seam, which takes more pulls than a limb; V-020). Skip patches and new body plans. | Claude Code | Opus 5.5, high | **ready** |
| S-011 | **Verifier** (needs web access to docs sites, Wikipedia, Defold and Esoteric pages, which this container blocked): V-007, V-013 (Nightmare Before Christmas half), V-017 (references); V-018, V-019 need a minimal repro on Godot 4.7.1, so pair them with a Build session. | Claude Code (with web) | Sonnet 5.5, medium | **ready** |

**In tandem now:** S-005, S-008 and S-011 (touching plan docs only),
with S-012 (the build). S-008 may answer Q-036; if so, S-012 follows it.

## Blockers

None.

## Where to look

- Decisions, one line each: `decisions/README.md`.
- Open questions and claims to check: `open-questions.md`, `verify.md`.
  `check.py` prints totals.
