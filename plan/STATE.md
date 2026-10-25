# State

Updated: 2026-10-25 (physics test built and live; Mikey plays it, S-004 next)

Rewrite this file, don't add to it. Keep it under 100 lines.

## Phase

**Gate A passed** (2026-10-25). The first playable
(`prototypes/03-first-playable.md`, B1 to B11) is built and live, as a
sketch (0021). Mikey wants a better-built slice of the first version,
with real mini games (proposed 0043) that any friend can play, its body
changing how (proposed 0044). Those stand on physics, so the physics
test (`prototypes/04-physics-test.md`) came first; it is built and live.

## Agenda

How it works: `kit/RULES.md` section 1. Mikey starts a session by
saying "Run S-NNN" in a new chat of the kind, model and effort shown.
A session ends by deleting its row, flipping rows it unblocks to
**ready**, and adding follow-ups at the bottom. Next number: S-012.

| # | Session | Where | Model, effort | Status |
|---|---|---|---|---|
| S-004 | **Brain** with Mikey: sign or change proposed 0043 and 0044, answer Q-032, and take his playtest notes on the first playable (B12) into `prototypes/03-first-playable.md`; his physics test impressions too, if he has played it (S-005). | Project | Opus 5.5, medium | **ready** when Mikey is |
| S-005 | **Mikey plays** the physics test on his phone and in the browser (`#physics`, `#physics&three`), judging by `prototypes/04-physics-test.md`. Live on the web build. | (phone) | (no model) | **ready** |
| S-006 | **Red team:** the physics test's result, 0043, 0044 and the body-ability framework, before any mini game is built on them. A fresh chat that didn't build it. | Claude Code | Fable 5.1, high (or Opus 5.5) | waiting on S-004, S-005 |
| S-007 | **Build:** the skateboard grind course, the first real mini game. | Claude Code | Opus 5.5, high | waiting on S-006 |
| S-008 | **Brain** with Mikey: which mini games, what each gives, where they happen; friend looks (fur, head shapes). | Project | Opus 5.5, medium | waiting on S-004 |
| S-009 | **Cleanup:** the regular one (RULES section 1). | Claude Code | Sonnet 5.5, medium | waiting until `check.py` says one is due |
| S-011 | **Verifier** (needs web access to docs sites, Wikipedia, Defold and Esoteric pages, which this container blocked): V-007, V-013 (Nightmare Before Christmas half), V-017 (references); V-018, V-019 need a minimal repro on Godot 4.7.1, so pair them with a Build session. | Claude Code (with web) | Sonnet 5.5, medium | **ready** |

**In tandem now:** S-005 and S-004 (whenever Mikey wants);
they touch different files.

## Blockers

None.

## Where to look

- Decisions, one line each: `decisions/README.md`.
- Open questions and claims to check: `open-questions.md`, `verify.md`.
  `check.py` prints totals.
