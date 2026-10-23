# What "done" means

"10/10" is a checklist, not a feeling. A doc moves up the status
ladder only when every box for that step is ticked. Any session can
grade a doc against this file; only Mikey ticks the last box.

## System doc (`systems/<name>/README.md`)

**idea → draft**
- [ ] Every section of `kit/templates/system.md` is filled or says
      "none yet".
- [ ] "In charge of" is one sentence, and "Not in charge of" names
      the neighbouring systems that own the nearby things.

**draft → review**
- [ ] Every term used is in `glossary.md`, with the same meaning.
- [ ] Every row in "Talks to" also appears, the other way round, in
      the other system's doc and in `systems-map.md`.
- [ ] Every rule is concrete enough to prototype: someone could build
      it from the doc without asking what it means.
- [ ] Each pillar in `00-vision/pillars.md` is checked in "Pillar
      check", and any tension is named.
- [ ] Open questions are in `open-questions.md`, not buried in prose.
- [ ] Every factual claim is cited or tagged `[unverified]` with a line
      in `verify.md`.
- [ ] Under its size budget.

**review → locked**
- [ ] A red team by a **different model** than the one that drafted it
      found no blockers, or each blocker is fixed or answered.
- [ ] No `[unverified]` tags remain.
- [ ] No open question still blocks it.
- [ ] Mikey read it start to finish and it says what he means. **(Mikey)**

**locked → prototyped**
- [ ] The prototype that tested it has a filled-in result in
      `prototypes/`.
- [ ] The doc now says what the prototype taught, with a link.

## Vision doc (`00-vision/*.md`)

- [ ] Fits on one screen (pitch) or is under its budget (others).
- [ ] Every pillar is a test: you could hold a feature up to it and
      say yes or no.
- [ ] Non-goals name real temptations, not obvious ones.
- [ ] First playable lists what's in **and** what's deliberately out.
- [ ] Red team found no contradictions between vision docs.
- [ ] Mikey says it's the game he wants to make. **(Mikey)**

## Decision (`decisions/NNNN-*.md`)

- [ ] States the decision in one sentence.
- [ ] Gives the reason, and at least one alternative with why it lost.
- [ ] Names its consequences: what it makes easier and harder.
- [ ] Signed by Mikey with a date, if accepted.

## Systems map (`systems-map.md`)

- [ ] Every system appears.
- [ ] Every connection says what flows, in which direction, and when.
- [ ] Matches the "Talks to" tables in every system doc.
- [ ] No system both owns and borrows the same data.

## A planning phase

A phase is done when every doc it produced is `locked`, its gate review
in `reviews/` says **go**, and `STATE.md` points at the next phase.
