# State

Updated: 2026-11-15 (S-206: lead types, the cycle and the play gate, R27 and R28)

Write this file again each time. Do not add to the end (90 lines at most). One line for each row. The full text of each row is in `rows.md`.

## Phase
**Demo 1** (`demo-1.md`) is built: walk, jump, climb, obstacles, tumble, seams and the smoke edge in one biome. It waits for Mikey's feel notes (S-137).
**Demo 2** (`demo-2.md`, R24, R25) is in work: lakes, rivers, waterfalls, walkable caves and the cave light are on `main` (build 2e2d5f8). Swimming (S-163) is the last row before Mikey's first look (S-167).
**Speed** on the Intel GPU: the Benchmark needs about 12.5 ms. Open views of lakes and cave mouths need 15 to 18 ms before any water (limit 16.7 ms). Water costs 1.1 to 2.2 ms (budget 1.0). The numbers are in `play-notes.md`.

## Cycle (`sessions/lead.md`, R27, R28)
Builder → Speed → Hygiene → Mikey plays → Feel → Bird's-eye. One lead at a time. Research runs while the cycle waits.

- **Next lead: Builder.** Wave: S-163, and beside it S-187, S-195 and S-186, one after the other.
- **Play gate: open** (1 play row is ready: S-137). It closes when S-163 is on `main` (S-167 becomes ready).
- Then: the Speed lead (S-182, S-183, S-181, S-205), the Hygiene lead, and Mikey's play.

Next number: S-212. A worker numbers its new rows from the range in its brief.

## Mikey plays (`sessions/play.md`). Do all ready rows in one sitting.
| # | Row | Status |
|---|---|---|
| S-137 | Demo 1 feel on the Intel GPU: 10 minutes, one line on the ground, scatter, walk, climb, smoke | **ready** |
| S-167 | First look at water and caves, and the new water look (was S-184) | waiting on S-163 |
| S-173 | The demo 2 playtest: falls, cave light, pools, far walls | waiting on S-168 to S-172 |
| S-143 | The same as S-137 on the Vivobook | waiting on the Vivobook |

## Builder (`sessions/lead-builder.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-163 | Swimming and wet fur | Sonnet 5.5, high | **ready**: first |
| S-187 | Dev menu: caves on and off, go to the nearest mouth | Sonnet 5.5, low | **ready** (check `bear.rs` against S-163) |
| S-195 | The cave light on the water | Sonnet 5.5, low | **ready** |
| S-186 | The cave rim looks like the ground | Sonnet 5.5, medium | **ready** |
| S-168 | Pools in caves | Sonnet 5.5, medium | waiting on S-195 |
| S-169 | The far walls of big chambers | Opus 5.5, high | **ready** |
| S-172 | Obstacles with water and caves | Sonnet 5.5, high | **ready** |
| S-201 | Look at the falls and tune them | Sonnet 5.5, medium | **ready** |
| S-124 | Playtest replay | Opus 5.5, high | **ready** |
| S-170 | Ragdoll, lost limbs and swim gaits in water | Sonnet 5.5, high | waiting on S-163 |
| S-171 | Autoplay for water and caves | Sonnet 5.5, medium | waiting on S-163 |
| S-204 | Falls in the autoplay | Sonnet 5.5, low | waiting on S-171 |
| S-203 | The bear in a fall | Sonnet 5.5, medium | waiting on S-163, S-167 |
| S-191 | Longer rivers, if Mikey wants them | Sonnet 5.5, medium | waiting on Mikey (S-167) |
| S-202 | True cliffs for the feature falls, if Mikey wants them | Opus 5.5, high | waiting on Mikey (S-167) |

## Speed (`sessions/lead-speed.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-182 | A water sweep script | Sonnet 5.5, low | **ready**: first |
| S-183 | Why a lake view without water costs 14.5 ms | Sonnet 5.5, medium | **ready** |
| S-181 | Cut the water to 1.0 ms or less | Sonnet 5.5, medium | **ready** (after S-182) |
| S-205 | A Cave tour without caves, for the base cost | Sonnet 5.5, medium | **ready** |

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
| S-174 | With Mikey: tune demo 2 from S-167, add rows S-175 to S-179 | Opus 5.5, medium | waiting on S-167 |
| S-192 | Tune the river look | Sonnet 5.5, medium | waiting on S-167 |

## Hygiene (`sessions/lead-hygiene.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-207 | Make the old briefs and docs agree with the plan | Sonnet 5.5, medium | **ready** |
| S-208 | The weak spots of the first demo 2 wave become rows | Sonnet 5.5, medium | **ready** |
| S-134 | Fix what the nightly autoplay finds | Sonnet 5.5, medium | **ready** (the nightly report exists) |

## Research (`sessions/lead-research.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-209 | Move `plan/research/` (15 notes) to the wiki | Sonnet 5.5, medium | **ready** |
| S-210 | How a seed selects looks, sizes and timings (`vision.md`) | Opus 5.5, medium | **ready** |

## Bird's-eye (`sessions/lead-birdseye.md`)
| # | Row | Model, effort | Status |
|---|---|---|---|
| S-211 | Sort the inbox, the "Proposed" and "Assumed" lists; check the direction | Fable 5.2, medium | after the first Feel lead |

## Blockers
None. The build does not wait. The taste questions for Mikey are in `HANDOFF.md`.
