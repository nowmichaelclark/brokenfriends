# Build session: make one agenda item playable

**Tier:** S (a strong model). **With:** nobody, or Mikey checking in.
Claude Code only (it runs the game and pushes).

## Purpose

Build the one thing your agenda row names, usually a prototype step,
so Mikey can play it. The build is a sketch (0021): the plans lead.

## Read first

`STATE.md` (your row), `game/README.md`, and the prototype doc your row
names. Then only the code you'll change.

## How to run it

1. Restate the "done when" from the prototype doc. If there isn't one,
   write it into the doc first.
2. **Pay off code warnings first.** `check.py` warnings about scripts
   you will touch (over 600 lines) are split before you add to them.
   Delete sketch code your change replaces.
3. Build in small steps; run the game after each and look at it
   (screenshots under `xvfb-run`, see `game/README.md`, "Testing").
4. A design question the plans haven't settled: build the simplest
   version, list it under "Assumed in this build" in `game/README.md`,
   and ask Mikey. Never record it as a decision.
5. Speed: compare `#perf` at 4x slowdown before and after.

## Done when

- The row's "done when" holds, shown by a test or screenshots.
- The playthrough test ends `FLOW OK`, extended to cover the change.
- Web and Android builds pass on GitHub.
- `game/README.md` says what the build is now and what it assumes (not
  a history of the steps).
- `check.py` shows no new warnings from your files.
- The prototype doc and the agenda row are updated (RULES section 1).

## Don'ts

- Don't plan beyond your row. Ideas go to `inbox/` as one line each.
- Don't commit screenshots, builds or debug output.
- Don't raise a limit to make a file fit.
