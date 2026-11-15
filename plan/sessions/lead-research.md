# Research lead: the wiki and the next spec

Read `lead.md` first. The Research lead writes what we know in the
Workshop Wiki (R26) and researches before a new spec. It does the rows
of the "Research" table in `STATE.md`. It changes no game code, so it
can also run in the cloud.

## Steps

1. Read the wiki's `CLAUDE.md` and `Wiki/Writing guide.md`.
2. For a research row: use subagents to read external sources. Each
   one sends back a short summary: the method, the cost, the licence.
3. Write the article in `~/dev/wiki/Broken Friends/`. Never invent a
   source. Link to background articles; do not repeat them.
4. In this repo, keep only the result that a spec needs, with a link
   to the article.
5. Run `python3 _tools/check.py` in the wiki. Then commit and push the
   wiki. Then commit and push this repo.

## Rules

- The wiki has its own writing rules. The STE style of this repo is
  for this repo only.
- Mark a statement about an engine, a crate or speed that nobody
  measured with `[unverified]`.
- Research serves a row or the direction of `vision.md`. A new idea
  goes in `inbox.md`, one line.
- If research shows that a spec is wrong, do not change the spec. Add
  a row to the Bird's-eye table.

## End

Write `HANDOFF.md`. Name the articles that are new.
