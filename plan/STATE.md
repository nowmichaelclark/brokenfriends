# State

Updated: 2026-10-25 (first playable built; physics test next)

Rewrite this file, don't add to it. Keep it under 100 lines.

## Phase

**Gate A passed** (review deleted; see git history). The first playable
(`prototypes/03-first-playable.md`, B1 to B11) is built and live, as a
sketch (0021). Mikey wants a better-built slice of the first version,
with real mini games (proposed 0043) that any friend can play, its body
changing how (proposed 0044). Those stand on physics, so the physics
test (`prototypes/04-physics-test.md`) comes first.

## Agenda

How it works: `kit/RULES.md` section 1. Mikey starts a session by
saying "Run S-NNN" in a new Claude Code chat with the model and effort
shown. A session ends by marking its row done (then deleting it),
flipping rows it unblocks to **ready**, and adding follow-ups at the
bottom. Next number: S-010.

| # | Session | Model, effort | Status |
|---|---|---|---|
| S-001 | **Build:** the physics test (`prototypes/04-physics-test.md`). First split `friend_view.gd` and `tent.gd` (over 600 lines) and trim `game/README.md` to what the build is now. | Opus 5.5, high | **ready** |
| S-003 | **Verifier:** V-014 to V-016 (are the lean limits right?) and V-001, V-009, V-010 (physics claims S-001 relies on). Change the limits in `kit/RULES.md` and `kit/tools/common.py` to what the evidence supports. | Sonnet 5.5, high | **ready** |
| S-004 | **Brain** with Mikey: sign or change proposed 0043 and 0044, answer Q-032, and take his playtest notes on the first playable (B12) into `prototypes/03-first-playable.md`. | Opus 5.5, medium | **ready** when Mikey is |
| S-005 | **Mikey plays** the physics test on his phone and in the browser, judging by `prototypes/04-physics-test.md`. | (no model) | waiting on S-001 |
| S-006 | **Red team:** the physics test's result, 0043, 0044 and the body-ability framework, before any mini game is built on them. A fresh chat that didn't build it. | Fable 5.1, high (or Opus 5.5) | waiting on S-004, S-005 |
| S-007 | **Build:** the skateboard grind course, the first real mini game. | Opus 5.5, high | waiting on S-006 |
| S-008 | **Brain** with Mikey: which mini games, what each gives, where they happen; friend looks (fur, head shapes). | Opus 5.5, medium | waiting on S-004 |
| S-009 | **Cleanup:** the regular one (RULES section 1). | Sonnet 5.5, medium | waiting on three more sessions after S-002 (done 2026-10-25) |

**In tandem now:** S-001 and S-003 touch different files and can
run at the same time; S-004 too, whenever Mikey wants.

## Blockers

None.

## Where to look

- Decisions, one line each: `decisions/README.md`.
- Open questions and claims to check: `open-questions.md`, `verify.md`.
  `check.py` prints totals.
