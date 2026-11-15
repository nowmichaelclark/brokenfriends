# Hygiene lead: tech debt and a clean repo

Read `lead.md` first. The Hygiene lead cleans after a wave. It does
the rows of the "Hygiene" table in `STATE.md`. It does not add
features and does not change how the game plays.

## What to look at

- **Weak spots.** Read the weak spots of the commits since the last
  Hygiene lead. Each one gets a row, a line in `inbox.md`, or a fix.
- **Docs.** `game/README.md` tells what the build is now. The briefs,
  `tech.md` and the specs agree with `decisions.md`.
- **`STATE.md` and `rows.md`.** No row waits on a row that is gone.
  Each row is in both files. `STATE.md` is below 90 lines.
- **Tests and CI.** Flaky tests, slow tests, things that only CI
  builds, warnings in the headless load, the nightly autoplay report.
- **Code.** Dead code, copies of the same logic, values that are not
  `#[export]`, `Default` values that differ from the exported ones.
- **Disk.** Old worktrees in `~/dev/bf-wt/`, old `target/` folders.

## Rules

- A change must keep the same world for the same seed. Run the tests.
- If a fix is larger than about one hour, make it a row with a model
  and an effort. Do not start it in the same session.
- Do not delete Mikey's files, the archive, or a playtest zip.
- This lead can use workers. Rows that change only docs can run
  together with rows that change code.

## End

Write `HANDOFF.md`. Say in it how many weak spots became rows. The
next step is usually Mikey's play, then the Feel lead.
