---
status: idea
last_reviewed: 2026-10-23
depends_on: []
feeds: [friend-body, condition, needs-growth, play, expeditions, audio-music]
---
# Species and habitats

## In charge of

The kinds of friend, what makes each different, and the places they
live (land and water), including what each place does to them.

## Not in charge of

- The body mechanics shared by all species: `friend-body`.
- Condition rules: `condition` (species only set their sensitivities).

## Seed notes (from Mikey's first ideas, unconfirmed)

- Friends that thrive in water and friends that thrive on land.
- Claude's suggestion: store each species as a data file (Godot calls
  these Resources), so a new species means filling in values, not
  writing code (V-002).

## First questions

- How many species at first playable?
- How does a stuffed friend thrive in water?
- Can species mix or breed?

## Sources

- Godot docs, "Resources" (godot-docs `tutorials/scripting/resources.rst`, master branch, checked 2026-10-25): the Inspector creates and edits custom Resource files, saved as `.tres`. V-002.
