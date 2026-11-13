---
date: 2026-10-25
session: cleanup
model: Claude Sonnet 5.5 (Claude Code)
---
# Cleanup: S-009, ticked verify items and old notes

## Changed

- `verify.md`: nine ticked items cleared, summarized here:
  - V-001 true: Godot 4.7.1 has `SoftBody3D`, no 2D soft body.
  - V-002 true: custom Resources are `.tres` files edited in the Inspector.
  - V-006 true: 4.7.1 has Skeleton2D, Bone2D, Polygon2D weights, joints, shaders, particles.
  - V-009 true with conditions: the `Rapier2D` setting works, but Rapier has no asymmetric collisions.
  - V-010 true: 2D joints have no break force; tearing is scripted.
  - V-012 true: Trauma Center's stylus is the scalpel and suture tool.
  - V-014 partly true: under 200 lines per CLAUDE.md is Anthropic advice; the 450 total is a judgment call.
  - V-015 unclear: the 600-line script limit stays a judgment call.
  - V-016 true for history (git keeps files while a ref reaches them); folder caps are judgment calls.
- `verify.md`: V-004, V-005, V-008 (ticked) stay, because accepted decisions 0003, 0004, 0015 and 0016 still carry `[unverified]` tags citing them. A Verifier clears both together.
- `log/`: four oldest notes deleted (git keeps them).
- No open question was removed: none is answered by a decision or doc. `inbox/`, `research/` and `reviews/` are empty.

## Decided by Mikey

- Nothing.

## Still open

- Glossary not audited against every doc (see Weak spots in the reply).

## Next

- S-012, S-008 and S-011 in tandem (STATE.md order).
