# For Claude Code sessions

This repo is **Broken Friends**. Plans live in `plan/`; the Godot
project lives in `game/`.

## Planning sessions

Read `plan/START-HERE.md`, `plan/kit/RULES.md` and `plan/STATE.md`
before anything else. "Run S-NNN" means the agenda row in `STATE.md`;
its type names the brief in `plan/kit/sessions/` (Build sessions:
`build.md`). Paths inside the plan docs are
relative to `plan/`.

Differences from chat sessions:

- Edit files directly instead of returning `=== FILE ===` blocks.
- Run `python3 plan/kit/tools/check.py` before you finish, and fix what
  it reports in the files you touched.
- Commit once per session: `<Session type>: <what changed>` (`check.py`
  counts these to pace Cleanup sessions). `git pull --rebase` before
  pushing; other sessions may be running in tandem.
- Agents for some session types live in `.claude/agents/`.

## Game code

Building started early, alongside planning (decision 0019). Read
`game/README.md` first: it has the layout, the rules the code follows,
and what this build assumes that Mikey hasn't confirmed yet.

- Godot 4.7.1, typed GDScript, Compatibility renderer, portrait.
- Keep data (`game/data/`) separate from drawing; announce changes on
  the `Events` bus.
- No image or sound files: draw and synthesize in code.
- A design question the plans haven't settled: build the simplest
  version, list it under "Assumed in this build" in `game/README.md`,
  and ask Mikey. Don't record it as a decision.
- Before pushing game changes, run the playthrough test
  (`game/README.md`, "Testing") and make sure it ends with `FLOW OK`.
- Pushing to `main` rebuilds the web version on GitHub Pages
  (`.github/workflows/web.yml`).
