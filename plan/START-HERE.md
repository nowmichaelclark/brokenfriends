# Start here

Read this first in every session, whatever model you are.

## What this is

The plans for **Broken Friends**, a 2D, mobile-first creature-care game.
Planning only: no game code yet. Mikey is the designer and the only one
who decides. You help him think, draft, check and criticize.

The plans live in the `plan/` folder of the `brokenfriends` repo. Every
path in these docs is relative to `plan/`.

## Your job

Your session brief (in `kit/sessions/`) says what kind of session this
is. Mikey's task, at the top of your pack, says what to do today. Do
that one job. If the task is unclear, ask before writing anything.

## The rules that matter most

Full rules: `kit/RULES.md`. The short version:

1. **Only touch the files the task names.** Anything else you notice
   goes in your "Weak spots" list or `open-questions.md`.
2. **Return whole files** between `=== FILE: path ===` and
   `=== END FILE ===` lines, all inside one code block fenced with
   `~~~~` lines. Never snippets, never "rest unchanged".
3. **Tag technical claims.** Anything about engines, tools, platforms
   or performance that you have not verified gets `[unverified]` after
   it, and a line in `verify.md`.
4. **Propose, don't decide.** Design choices are Mikey's. Offer two or
   three options with trade-offs. Decisions exist only once Mikey says
   so, and they are signed with his name and the date.
5. **Never edit `MIKEY.md` or a doc whose status is `locked`.**
6. **Replace, don't append.** Edit docs in place; git keeps history.
   `STATE.md` is rewritten each time, never added to.
7. **Use glossary terms exactly.** New term? Add it to `glossary.md`.
8. **Be a critic, not a cheerleader.** End every reply with
   **Weak spots**: at least three honest problems you see.

## Where things live

| Need | Look in |
|---|---|
| Where planning stands, what's next | `STATE.md` |
| What the game is and isn't | `00-vision/` |
| A game system | `systems/<name>/README.md` |
| How systems connect | `systems-map.md` |
| What a word means | `glossary.md` |
| What's been decided | `decisions/` (status `accepted`) |
| What's still open | `open-questions.md` |
| Claims awaiting a check | `verify.md` |
| What "done" means | `kit/RUBRIC.md` |
| Phases and gates | `kit/PHASES.md` |
| Templates | `kit/templates/` |

## Status ladder for docs

`idea` → `draft` → `review` → `locked` → `prototyped`

Only Mikey sets `locked`. A locked doc reopens only through a decision
record that sends it back to `draft`.
