# State

Updated: 2026-11-15 (S-206: seven lead types, the cycle, the play gate, the plan check, the bug tools)

Write this file again each time. Do not add to the end (90 lines at most). One line for each row. The full text of each row is in `rows.md`. `tools/check-plan.py` checks both files.

## Phase
**Demo 1** (`demo-1.md`) is built: walk, jump, climb, obstacles, tumble, seams and the smoke edge in one biome. It waits for Mikey's feel notes (S-137).
**Demo 2** (`demo-2.md`, R24, R25) is in work: lakes, rivers, waterfalls, walkable caves and the cave light are on `main` (build 2e2d5f8). Swimming (S-163) is the last row before Mikey's first look (S-167).
**Speed** on the Intel GPU: the Benchmark needs about 12.5 ms. Open views of lakes and cave mouths need 15 to 18 ms before any water (limit 16.7 ms). Water costs 1.1 to 2.2 ms (budget 1.0). The numbers are in `play-notes.md`.

## Cycle (`sessions/lead.md`, R27, R28)
Builder → Speed → Fix → Hygiene → Mikey plays → Feel → Bird's-eye. One lead at a time. Research runs while the cycle waits. A Fix lead can run at any time.
- **Next lead: Fix**, for the bug tools S-213 to S-216 (`diagnostics.md`, "Finding bugs"). Then Builder. Wave: S-163, and beside it S-187 and S-186, one after the other.
- **Play gate: open** (1 play row is ready). It closes when S-163 is on `main` (S-167 becomes ready). Then the Builder does only rows with "–" or a ready play row in the "Play" column.

Next number: S-217. A worker numbers its new rows from the range in its brief.

## Mikey plays (`sessions/play.md`). Do all ready rows in one sitting.
| # | Row | Status |
|---|---|---|
| S-137 | Demo 1 feel on the Intel GPU: 10 minutes, one line on the ground, scatter, walk, climb, smoke | **ready** |
| S-167 | First look at water and caves, and the new water look (was S-184) | waiting on S-163 |
| S-173 | The demo 2 playtest: falls, cave light, pools, far walls | waiting on S-168, S-169, S-170, S-172 |
| S-143 | The same as S-137 on the Vivobook | waiting on the Vivobook |

## Builder (`sessions/lead-builder.md`). "Play" is the play row that shows the work to Mikey.
| # | Row | Play | Model, effort | Status |
|---|---|---|---|---|
| S-163 | Swimming and wet fur | S-167 | Sonnet 5.5, high | **ready**: first |
| S-187 | Dev menu: caves on and off, go to the nearest mouth | S-167 | Sonnet 5.5, low | **ready** (check `bear.rs` against S-163) |
| S-186 | The cave rim looks like the ground | S-167 | Sonnet 5.5, medium | **ready** |
| S-195 | The cave light on the water | S-173 | Sonnet 5.5, low | **ready** |
| S-168 | Pools in caves | S-173 | Sonnet 5.5, medium | waiting on S-195 |
| S-169 | The far walls of big chambers | S-173 | Opus 5.5, high | **ready** |
| S-172 | Obstacles with water and caves | S-173 | Sonnet 5.5, high | **ready** |
| S-201 | Look at the falls and tune them | S-173 | Sonnet 5.5, medium | **ready** |
| S-170 | Ragdoll, lost limbs and swim gaits in water | S-173 | Sonnet 5.5, high | waiting on S-163 |
| S-203 | The bear in a fall | S-173 | Sonnet 5.5, medium | waiting on S-163, S-167 |
| S-191 | Longer rivers, if Mikey wants them | S-173 | Sonnet 5.5, medium | waiting on Mikey (S-167) |
| S-202 | True cliffs for the feature falls, if Mikey wants them | S-173 | Opus 5.5, high | waiting on Mikey (S-167) |
| S-124 | Playtest replay (a tool) | – | Opus 5.5, high | **ready** |
| S-171 | Autoplay for water and caves (a tool) | – | Sonnet 5.5, medium | waiting on S-163 |
| S-204 | Falls in the autoplay (a tool) | – | Sonnet 5.5, low | waiting on S-171 |

## Speed (`sessions/lead-speed.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-182 | A water sweep script | Sonnet 5.5, low | **ready**: first |
| S-183 | Why a lake view without water costs 14.5 ms | Sonnet 5.5, medium | **ready** |
| S-181 | Cut the water to 1.0 ms or less | Sonnet 5.5, medium | **ready** (after S-182) |
| S-205 | A Cave tour without caves, for the base cost | Sonnet 5.5, medium | **ready** |

## Fix (`sessions/lead-fix.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-213 | Tool: play probes for the camera (`camera_clip`) | Sonnet 5.5, medium | **ready**: first |
| S-215 | Tool: the look tour (one clean picture of each kind of thing) | Sonnet 5.5, high | **ready** |
| S-214 | Tool: the world audit (shape rules on many seeds, no GPU) | Sonnet 5.5, high | **ready** |
| S-216 | Tool: the look review (`looks.md`, reference pictures, review step) | Opus 5.5, medium | waiting on S-215 |
| S-212 | Mikey's three bugs: square water edge, bright haze in deep caves, camera through the ground | Opus 5.5, high | waiting on S-213, S-214, S-216 |
| S-134 | Fix what the nightly autoplay finds | Sonnet 5.5, medium | **ready** (the nightly report exists) |

## Feel (`sessions/lead-feel.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-109 | With Mikey: the full demo 1, decide what to fix | Opus 5.5, medium | waiting on S-137 |
| S-126 | Tune the walk animation | Sonnet 5.5, medium | waiting on S-137 |
| S-130 | Tune the climbing hands and the gaits | Sonnet 5.5, medium | waiting on S-137 |
| S-131 | Tune the smoke edge | Sonnet 5.5, medium | waiting on S-137 |
| S-141 | Tune the scatter; find the hitches and the slow first run | Sonnet 5.5, medium | waiting on S-137 |
| S-150 | Tune the obstacles again | Sonnet 5.5, medium | waiting on S-137 |
| S-155 | Check the tear and tumble values | Sonnet 5.5, medium | waiting on the next playtest |
| S-174 | With Mikey: tune demo 2 from S-167, add the tuning rows | Opus 5.5, medium | waiting on S-167 |
| S-192 | Tune the river look | Sonnet 5.5, medium | waiting on S-167 |

## Hygiene (`sessions/lead-hygiene.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-207 | Make `tech.md`, the specs and `game/README.md` agree with the plan | Sonnet 5.5, medium | **ready** |
| S-208 | The weak spots of the first demo 2 wave become rows | Sonnet 5.5, medium | **ready** |

## Research (`sessions/lead-research.md`) and Bird's-eye (`sessions/lead-birdseye.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-209 | Research: move `plan/research/` (15 notes) to the wiki | Sonnet 5.5, medium | **ready** |
| S-210 | Research: how a seed selects looks, sizes and timings (`vision.md`) | Opus 5.5, medium | **ready** |
| S-211 | Bird's-eye: sort the inbox and the open lists; check the direction and the cycle | Fable 5.2, medium | after the first Feel lead |
