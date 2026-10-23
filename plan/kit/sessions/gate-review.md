# Gate review: go or no-go

**Tier:** S (a strong model), with web search for Gates C and D.
**With:** nobody; Mikey reads the verdict.

## Purpose

Decide whether a phase's output is good enough to build on. This is
the expensive session, so its job is judgment, not drafting. Read
everything, write little.

## Pack with

Everything the phase produced, the vision docs, accepted decisions
(`--with-decisions`), `systems-map.md`, `glossary.md`, the phase's
red-team reviews, `kit/RUBRIC.md` and `kit/PHASES.md`.

## Questions by gate

- **Gate A (vision):** Is this one coherent game? Are the pillars real
  tests? Do the non-goals hold back the real temptations? Can one
  person build the first playable?
- **Gate B (architecture):** Does every piece of data have exactly one
  owner? Are system boundaries where change is likely? What would force
  a rewrite later? Is the Driftloom connection planned from the start?
  Which risky technical choices need a spike (a tiny test build) before
  anything else?
- **Gate C (feasibility):** Can every rule in the deep-pass docs be
  built in Godot as described? Check the risky ones against current
  Godot docs. What is the smallest prototype that answers the biggest
  question, and how will Mikey judge it?
- **Gate D (build queue):** Does the queue match the locked docs? Is
  each item one session's work, with yardsticks and stop conditions?

## Output

`reviews/YYYY-MM-DD-gate-<letter>.md` from `kit/templates/review.md`,
starting with a one-line verdict:

- **Go**
- **Go, after these fixes:** a short numbered list
- **No-go:** why, and what must happen first

Then the findings, the red-team findings you disagree with and why,
and **Weak spots**.
