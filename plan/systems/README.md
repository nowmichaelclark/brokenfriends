# Systems

One folder per game system, each with a `README.md` built from
`kit/templates/system.md`. A big system may split into extra docs in
its folder (each under 300 lines).

This list is a **first guess**. Phase 2 may merge, split, rename or cut
systems. Each stub says what the system is in charge of and where its
boundaries are, so ideas land in the right place from the start.

| System | In charge of |
|---|---|
| `creature-body` | The physical creature: parts, seams, skeleton, how it moves and comes apart |
| `condition` | What state the material is in: wet, moldy, dirty, torn, stuffing lost |
| `needs-growth` | Hunger, feeding, weight, energy, mood, growing up |
| `care-touch` | Everything the player does with a finger: poke, drag, feed, wash, dry, sew |
| `species-habitats` | Kinds of creature, land and water, and what each place does to them |
| `play` | Toys, minigames, and the skills creatures grow by playing |
| `expeditions` | Delivery quests: carrying, terrain, time away, rewards |
| `time-save` | The clock, catching up after time away, saving and loading |
| `progression` | Goals, unlocks and whatever the player earns or spends |
| `ui-ux` | Screens, menus, camera, touch controls, accessibility |
| `audio-music` | Driftloom music, creature themes, sound effects |
| `art-direction` | The look: style, palette, animation approach |
| `platforms-tech` | Engine, project layout, export targets, performance, test scenes |

Statuses are listed by `python3 kit/tools/check.py`.
