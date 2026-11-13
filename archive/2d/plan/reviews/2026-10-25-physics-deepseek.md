---
kind: red-team
date: 2026-10-25
model: DeepSeek
target: [prototypes/04-physics-test.md, game/scenes/physics/ragdoll.gd, game/scenes/physics/skateboard.gd, game/scenes/physics/physics_play.gd, decisions/0025-parts-come-off-and-wear-out.md, decisions/0026-heads-and-the-dead.md, decisions/0044-the-body-is-the-controller.md, systems/play/README.md, systems/friend-body/README.md]
---
# Review: physics test, tearing/neck, body-ability framework (DeepSeek)

## Verdict

Red team: **4 blockers, 7 majors, 9 minors**.

The physics test is a real start, but the code is a **limb ragdoll**, not yet a
friend ragdoll. Do not build S-007's skateboard grind course on the current
neck, head-loss or body-ability state until the blockers are fixed or S-012/S-015
land.

## Findings

| # | Severity | Where | What's wrong | Suggested fix |
|---|---|---|---|---|
| 1 | blocker | `ragdoll.gd` `_check_jolt`, `tear`; `prototypes/04-physics-test.md` Result | The neck is explicitly skipped (`if id == "neck": return`). There is no `PartData`, picture, or headless state for `"neck"`, so `tear("neck")` would hit nulls. This contradicts 0025/0026 and Mikey's chosen fix: a very hard neck hit should count as a pull. | Add a head/neck seam with its own pull count (more than a limb), a `tear_head()` path that sets `data.headless`, removes the head body/joint, and a test. |
| 2 | blocker | `decisions/0026`, `systems/play`, `ragdoll.gd` | Headless life is not modeled: no data fields for a loose head, no "head without body dies after about a day", no sewing a head onto a body, no mid-game head-loss end or leniency. A builder must invent all of it. | Define the head/headless state and transitions in `friend-body` and `play`; add minimal data and a physics-test hook. |
| 3 | blocker | `verify.md` V-020; `ragdoll.gd` `setup`; `skateboard.gd` | V-020 is still open. The code leaves the neck gap: head and body are separate convex shapes with collision exceptions, and the board deck is a solid rectangle with no neck guard. No repro test exists. The Result says Mikey chose "fix the snag", but the code does not. | Add a neck collar/filler collision shape or change the deck shape; add a repeatable "board into neck" test; close V-020 with evidence. |
| 4 | blocker | `decisions/0044`, `systems/play`, `systems/friend-body`, `ragdoll.gd` | The ability framework is prose only. No ability names are defined as data, no ranges, no derivation from body plan or parts, no API. The code exposes mass, size, wetness and stuffing; it does not expose grip, push, jump, balance or float. A mini game cannot ask for them. | Add an ability block to `friend-body` (names, ranges, derivation, owner); expose it on `Ragdoll`; start with skateboard's grip, push and balance. |
| 5 | major | `ragdoll.gd` `LIMBS`, `_add_limb` | The rig is hardcoded to two arms and two legs. Four legs, insect legs, tentacles, floating and oozing body plans (0044/0045) cannot be represented; non-arm/leg parts are treated as arms. | Build the seam/limb list from `FriendData`; add a body-plan rig; test at least one non-biped in `#physics`. |
| 6 | major | `decisions/0025`; `ragdoll.gd` `_pulls_needed`, `tear` | 0025 says any part, about three pulls, more for a strong part. The code only tears limbs, has no strong-part property, no body/torso tear, and relies on `part.pulls_needed()` not in the pack. | Put pull counts per part and quality in data; include head and body; document the numbers; test healthy, strong and rotting parts. |
| 7 | major | `ragdoll.gd` `_check_jolt` | Impact is judged by the part's own pre-impact speed. A fast skateboard hitting a still neck or limb will not count as a pull, because the part was not moving fast before contact. V-020 is exactly a moving board. | Detect relative impact or contact impulse, or add an explicit snag event; test a board into a stationary friend. |
| 8 | major | `prototypes/04-physics-test.md` Result | The prototype did not answer its own feel questions: tiny/huge/soaked/legless differences, whether tearing is funny and fair, and phone smoothness are all "not reported". The speed table is software-rendered at 4x slowdown, not the Moto. | Mikey playtests the variants and the phone before S-007; record answers in Result; keep speed as S-012's job but state it. |
| 9 | major | `prototypes/04-physics-test.md` smallest build; `physics_play.gd` variant; `ragdoll.gd` | "A friend with no legs sits on the board" is not implemented. There is no sit pose, balance aid, grip or board attach; active ragdoll is out of scope. A legless friend will likely flop, not ride. | Define a minimal sit/pose aid for board games, or accept the flop and test it; add balance and grip to abilities. |
| 10 | major | `physics_play.gd` `_unhandled_input` | Only `InputEventScreenTouch` and `InputEventScreenDrag` are handled. Decision 0003 says mouse must work and browser is a target; desktop browser play will not drag. | Add `InputEventMouseButton` and `InputEventMouseMotion` paths. |
| 11 | major | `systems/friend-body/README.md`, `systems/play/README.md`, `systems-map.md` | `friend-body` is still `idea`, has no "Talks to", no ability/part/seam spec; `play` depends on it but the connection is not in `systems-map.md`. Ownership of ability values is unclear. | Draft `friend-body` with the part/seam/ability data; add Talks to both ways and to `systems-map.md`. |
| 12 | minor | `skateboard.gd` `place` | `place` teleports jointed bodies and wheels without recreating the pin joints. Whether Godot keeps the joint anchors valid after a teleport is not checked [unverified]; reset may snap or stretch. | Test reset repeatedly; if needed recreate the pins or freeze bodies during placement. |
| 13 | minor | `physics_play.gd` `spawn` | Old friends are `queue_free()`d, then new ones are added in the same frame; the old bodies can still collide for a frame. | Remove them immediately or defer the spawn one frame. |
| 14 | minor | `ragdoll.gd` `tear`, `settled`, `speed` | A torn limb stays in `bodies`, so `settled()` and `speed()` include it. A test can wait forever on a free-falling torn part. | Keep active and torn arrays separate, or exclude torn bodies from settle and speed. |
| 15 | minor | `decisions/0025`; `ragdoll.gd` `tear` | `tear` reduces stuffing and spawns fluid on every tear. 0025 says donation takes nothing else; open-hole leak and rot is `condition`'s job. This is an undocumented guess. | Decide whether a tear leaks stuffing; if yes, give a number and owner; if no, remove the reduction. |
| 16 | minor | `glossary.md`, `systems/play`, `prototypes/04-physics-test.md` | "minigame" and "mini game" are both used; neither is a glossary term. "Seam" still calls the hard-landing pull "a guess", while the prototype treats it as a rule. | Add **Mini game** to the glossary; update **Seam** once the hard-landing rule is decided or proven. |
| 17 | minor | `prototypes/04-physics-test.md` Done when | "Web and Android builds pass on GitHub" is a done condition; the Result does not say whether they pass. | Report build status, or mark it unchecked. |
| 18 | minor | `ragdoll.gd` `setup`, `_seam` | `setup` assumes the `Ragdoll` is at origin and bodies are unrotated; `_seam` uses `a.position + at_a`, not a global transform. If a mini game parents the ragdoll under a transformed room, seams will be wrong. | Use global transforms for seam placement, or document and enforce "Ragdoll must stay at origin". |
| 19 | minor | `systems/play` | "A head that comes off mid-game usually ends the game" has no definition: fail, close, death, or a leniency window? | Define the states and the leniency rule before a mini game implements it. |
| 20 | minor | `decisions/0044` | "Every mini game can be played by any friend" and "when a body truly can't do a game (rare)" read as contradictory. | Say "almost every" or define the rare exception test. |

## Checks that passed

- Tearing is scripted, not from a joint breaking force, matching V-010 and the prototype's claim.
- Weight, bounce, wetness, stuffing, missing parts, collision exceptions, three-friend spawn and debug variants are present in code.
- The test's playthrough covers drop and settle, loose-seam tear, board roll and three friends; no contradiction found with 0002 (2D), 0005 (built-in physics first), 0019/0021 (sketch), 0022 (friend word), or 0031 (front/back not involved).
- No weapons or combat, and no text-heavy design, was introduced.
- I edited nothing.

## Rubric grade

- `prototypes/04-physics-test.md`: no rubric row for prototypes. It has a question, smallest build and result, but its own judgment questions are unanswered and V-020 remains. Not ready to be a base.
- `decisions/0025`, `0026`, `0044`: decision rubric met (decision, context, why, alternative, consequences, signed), but 0025/0026/0044 consequences point at systems still `idea`; 0026 has Q-026 open.
- `systems/play/README.md`: idea→draft partly; draft→review fails concrete rules, Talks to, pillar check and verify tags; the mini game list is concrete, the ability framework is not.
- `systems/friend-body/README.md`: idea→draft partly; draft→review fails on concrete part/seam/ability rules, Talks to, pillar check and verify tags; "first questions" still open.
- `systems-map.md`: empty; cannot grade beyond idea.
- Code scripts: no rubric row; code has no automated neck, head or ability tests and hardcodes biped limbs.

## Weak spots in this review

- I could not run Godot or reproduce V-018, V-019 or V-020; code behavior and joint teleport behavior are inferred [unverified].
- I did not see `FriendData`, `PartData`, `FriendArt`, `FriendRig`, `Baked`, `Flags`, `Save` or `FxLayer`; some claims rely on those APIs (`pulls_needed()`, `headless`, `is_open`).
- No phone performance or feel data exists in the pack; fun risks are judgment, not measurement.
- I focused on physics, the neck and the ability framework; I did not review every mini game idea or the art generator.
- Severity calls are mine. A blocker for a system doc may be a major for a throwaway sketch build.

<!-- kind: red-team -->
