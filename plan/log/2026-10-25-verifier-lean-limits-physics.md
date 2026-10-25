---
date: 2026-10-25
session: verifier
model: Claude Sonnet 5.5 (Claude Code)
---
# Verifier: lean limits and physics claims

## Changed

- `verify.md`: V-001, V-009, V-010, V-014, V-015, V-016 ticked with
  sources. The Godot docs site was unreachable, so the 4.7.1-stable
  class reference XML in the engine repo was read instead.
- `kit/RULES.md`, section 6: `[unverified]` tags replaced. 200 lines per
  hot-set file is Anthropic's advice; 450, 600 and the folder caps are
  marked judgment calls. Shallow-clone note added to "Git is the archive".
- `kit/tools/common.py`, `kit/tools/check.py`: new per-file hot-set cap
  of 200 lines (`HOT_SET_FILE_LINES`). No other limit changed.
- `decisions/0005-physics-builtin-first.md`: V-009 tag cleared; notes
  that Rapier behaves differently in two places.

## Decided by Mikey

- Nothing.

## Still open

- V-002, V-006, V-007, V-012, V-013 still open (S-010).

## Next

- S-001: it can rely on V-001 (no 2D soft body: fake squish) and V-010
  (joints don't break: script the tear).
