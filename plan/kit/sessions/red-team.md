# Red team session: find what's wrong

**Tier:** F, from a **different lab** than the model that drafted the
docs. **With:** nobody; Mikey reads the result.

## Purpose

Find the problems before they become code. You are not here to praise
or polish. A review that finds nothing has failed, unless it shows the
checks it made and why each passed.

## Pack with

The docs under review, the docs they depend on, `00-vision/`,
`glossary.md`, `systems-map.md`, accepted decisions
(`--with-decisions`), `kit/RUBRIC.md`.

## What to hunt for

1. **Contradictions** between docs, or with an accepted decision.
2. **Gaps:** things a builder would have to guess. Rules with no
   numbers, states with no way out, edge cases with no answer.
3. **Broken connections:** data one system expects that no system
   provides, or that two systems both claim to own.
4. **Wrong or shaky facts:** technical claims that are not tagged, or
   that you believe are false. Say what you believe and how sure you
   are.
5. **Pillar violations** and creeping scope: features that a solo
   developer can't finish, or that a non-goal rules out.
6. **Word drift:** terms used differently from `glossary.md`.
7. **Fun risks:** places where the plan is complete but you doubt it
   will feel good, and why.

## Output

One file, `reviews/YYYY-MM-DD-<target>-<model>.md`, from
`kit/templates/review.md`. Each finding has a severity:

- **blocker:** can't lock until fixed
- **major:** should fix before locking
- **minor:** worth fixing, not urgent

…plus where it is (file and section), what's wrong, and a suggested
fix. Do not edit the docs you review. End with your rubric grade for
each doc and **Weak spots** in your own review.
