---
status: accepted
date: 2026-10-23
decided_by: Mikey, 2026-10-23
supersedes:
superseded_by:
---
# 0001: Plan in docs first, mostly on cheap models

## Decision

The game is planned in this docs-only repo before any game code, mostly
in sessions on free or cheap models, with strong-model reviews at a few
gates.

## Context

Driftloom started as code and found its architecture later, which
meant a mid-project port and two tracks of work. Its planning docs also
grew append-only into thousands of lines. Mikey wants rigorous,
structured planning this time, at little or no cost.

## Why

- Planning mistakes are cheapest to fix on paper.
- Most planning is drafting and discussion, where Mikey's judgment
  matters more than model strength.
- Cheap models' weak spots (shaky facts, drift, agreeing too easily)
  are caught by the kit: tagged claims, a rubric, cross-model red
  teams and strong-model gates.

## Alternatives rejected

- **Prototype first, plan as you go:** the Driftloom route; fast, but
  it caused the late architecture change.
- **Plan everything on strong models:** costs far more for little gain
  in the drafting work.
- **A complete 200-page plan before any prototype:** a toy-feel game
  can't be fully planned on paper; prototype checkpoints are built into
  the phases instead.

## Consequences

Easier: cheap iteration; a plan that sessions can execute later.
Harder: discipline is needed to keep docs small and verified.
