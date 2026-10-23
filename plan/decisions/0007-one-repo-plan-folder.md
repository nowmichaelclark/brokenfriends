---
status: accepted
date: 2026-10-23
decided_by: Mikey, 2026-10-23
supersedes:
superseded_by:
---
# 0007: One repo, plans in plan/, code in game/

## Decision

Plans and code share one repo, `nowmichaelclark/brokenfriends`: the
plans in `plan/`, the Godot project in `game/`.

## Context

Mikey asked whether the plans should get their own repo or a folder in
the game repo, and set the repo up with a `plan/` folder (was Q-008).

## Why

- Claude Code sessions building the game can read the locked plans
  next to the code.
- One history, one GitHub check, and links between plans and code work.
- Keeping the Godot project in its own folder stops the editor from
  scanning the planning docs.

## Alternatives rejected

- **A separate planning repo:** cleaner split, but plans and code drift
  apart, and build sessions need both. Only worth it if one should be
  private and the other public.
- **Plans at the repo root:** the game's front page would be the
  planning kit.

## Consequences

Repo-level files (`CLAUDE.md`, `.github/`, `.claude/`, `.gitignore`)
live at the top; everything planning lives in `plan/`, and paths in the
plan docs are relative to `plan/`.
