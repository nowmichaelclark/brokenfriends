# Phases and gates

Planning goes breadth first, then depth, with a prototype checkpoint
before each deep pass is trusted. Most sessions run on free or cheap
models. Strong models are used at the gates, mostly to **read**, which
costs far less than writing.

## Model tiers

| Tier | What | Used for |
|---|---|---|
| **F** (free or cheap) | Any chat model Mikey likes, e.g. DeepSeek or a free tier from any lab | Brain, Drafter, Red team, Research, Cleanup |
| **M** (mid) | A capable paid model, e.g. Claude Sonnet or Haiku | Verifier (needs web search), tricky drafts |
| **S** (strong) | A top model, e.g. Claude Opus | Gate reviews only |

Red teams use a model from a **different lab** than the one that wrote
the draft. Different models miss different things.

## Phase 0: the kit (done when `check.py` passes)

Mikey reads `kit/`, changes anything he dislikes, and pushes the repo.

## Phase 1: vision

Produce: `00-vision/pitch.md`, `pillars.md`, `non-goals.md`,
`references.md`, `first-playable.md`. Seed `glossary.md` with the
game's core words.

Sessions: Brain (F) with Mikey, back and forth until the pitch and
pillars feel right; Research (F) for references; Drafter (F);
Red team (F, different lab).

**Gate A (S):** one strong-model review of the five vision docs.
Contradictions, weak pillars, missing non-goals, scope that won't fit
a solo developer.

## Phase 2: shallow pass

Produce: every `systems/*/README.md` at `draft` (about half a page
each), `systems-map.md`, a fuller glossary. The systems list itself
may change: merge, split or cut systems as the vision demands.

Sessions: Brain (F) per cluster of systems, Drafter (F), Red team (F),
Cleanup (F).

**Gate B (S):** architecture review. Do the systems' boundaries make
sense? Does every piece of data have one owner? Will anything force a
rewrite later (the Driftloom lesson)? Is the Driftloom music connection
planned from the start?

## Phase 3: deep pass, the core trio

Produce: `creature-body`, `condition` and `care-touch` at full depth,
enough to build the first toy. Everything else stays shallow.

Sessions: Brain (F), Drafter (F), Red team (F), Verifier (M) to clear
all `[unverified]` tags in the trio.

**Gate C (S + search):** feasibility. Check each rule in the trio
against Godot's actual documentation. List what to build in the first
prototype and the yardsticks for judging it.

## Prototype 1: the toy

Not planning, but it closes the phase. One creature on Mikey's phone
that can be poked, dragged, torn, sewn and overfed. Mikey plays it, and
the results go in `prototypes/`. Docs are updated with what it taught
and moved to `prototyped`.

This is the free verifier: half an hour in Godot settles questions no
model can.

## Phase 4 onward: one cluster at a time

Repeat Phase 3's pattern (deep pass, verify, Gate C, prototype) for:

1. `needs-growth` and `time-save` (the pet layer)
2. `play`
3. `expeditions`
4. `audio-music` (Driftloom), `progression`, `ui-ux`

Order can change; Mikey decides.

## Final phase: build planning

Locked docs become a `QUEUE.md` for Claude Code sessions: numbered
items with a why, a what, yardsticks and stop conditions
(`kit/templates/queue-item.md`), under standing rules, like Driftloom's
queue but kept short (finished items archived).

**Gate D (S):** the queue matches the locked docs, and each item is
small enough for one session.

## Cost at a glance

| Sessions | Tier | Roughly how many |
|---|---|---|
| Brain, Drafter, Red team, Research, Cleanup | F | most of them |
| Verifier | M | one batch per deep pass |
| Gate reviews | S | four or five in all |
