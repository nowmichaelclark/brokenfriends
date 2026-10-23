# State

Updated: 2026-10-23 (moved into the brokenfriends repo)

Rewrite this file, don't add to it. Keep it under 100 lines.

## Phase

**Phase 0: the kit.** The planning kit lives in `plan/` of the
`brokenfriends` repo. Mikey reviews it next.

## Next steps

1. Mikey reads `START-HERE.md`, `kit/RULES.md`, `kit/PHASES.md` and
   changes anything he dislikes.
2. Mikey pushes the repo to GitHub.
3. First Brain session (tier F, DeepSeek): the pitch and the pillars.
   From `plan/`:
   `python3 kit/tools/pack.py brain 00-vision inbox --task "..." --copy`

## Blockers

None.

## Decided so far

- 0001 Planning method: docs first, cheap models, strong-model gates.
- 0002 The game is 2D only.
- 0003 Platforms: mobile first, also browser, maybe native desktop.
- 0006 The game is called Broken Friends.
- 0007 One repo: plans in `plan/`, the Godot project in `game/`.

Proposed, waiting on Mikey: 0004 Godot as the engine; 0005 Godot's
built-in physics first, Rapier only if needed.

## Open counts

See `open-questions.md` and `verify.md`. `check.py` prints totals.
