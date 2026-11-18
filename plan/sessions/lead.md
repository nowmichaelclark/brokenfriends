# Lead sessions: the cycle and the common rules

A lead is one Claude Code session on Mikey's laptop
(`~/dev/brokenfriends`). It runs on the laptop because only the laptop
can start Godot, play the game and measure speed (CLAUDE.md, "Speed
target"). There are seven types of lead. Only one lead runs at a time
(R27).

Mikey starts a lead with: "Start the <type> lead. Read
plan/sessions/lead.md." If he gives no type, use the type that
`STATE.md` names in "Cycle".

## The seven leads

| Lead | Brief | Job | Model, effort |
|---|---|---|---|
| Builder | `lead-builder.md` | Runs workers on feature rows | Opus 5.5, medium |
| Speed | `lead-speed.md` | Measures and cuts frame time, alone | Sonnet 5.5, high |
| Fix | `lead-fix.md` | Finds, shows and fixes bugs | Opus 5.5, medium |
| Feel | `lead-feel.md` | Turns Mikey's play into new defaults | Sonnet 5.5, medium |
| Hygiene | `lead-hygiene.md` | Docs, tests, CI, weak spots | Sonnet 5.5, medium |
| Research | `lead-research.md` | Wiki articles, research for a spec | Sonnet 5.5, medium |
| Bird's-eye | `lead-birdseye.md` | Sorts, checks the direction, plans | Opus 5.5, high |

## The cycle (R27)

1. **Builder:** one wave of feature rows, until the play gate closes
   or the ready rows end.
2. **Speed:** measure the wave on the Intel GPU. Cut what is above
   its budget.
3. **Fix:** fix the known bugs, so Mikey does not play them.
4. **Hygiene:** clean after the wave.
5. **Mikey plays** all ready play rows in one sitting.
6. **Feel:** read the zip and the notes, change the defaults, give
   Mikey a new build. His bugs become rows in the Fix table.
7. **Bird's-eye:** every week, or after a play that changes the plan.

The Research lead has no fixed place. Run it before a new spec, or
while the cycle waits for Mikey. A Fix lead can also run at any time,
when Mikey names a bug that he does not want to wait for. Short Fix
and Hygiene leads can be one session, one type after the other. A lead can stop early. Then it writes
in `STATE.md`, "Cycle", which lead is next.

## The play gate (R28)

Count the play rows with the status **ready**. If two or more are
ready, the gate is closed. The gate opens when Mikey has played.

Each Builder row has a "Play" column: the play row that shows the work
to Mikey, or "–" for a tool that he does not see. While the gate is
closed, the Builder lead starts a row only if its "Play" column has
"–" or a play row that is ready now. Thus it can finish the wave that
Mikey is about to play, and it does not start the next one. All other
leads can run.

Write the gate status in `STATE.md`, "Cycle", as `**Play gate: open**`
or `**Play gate: closed**`. `tools/check-plan.py` counts the rows and
fails if the line is wrong.

## Start

1. Run `git pull`. Read `CLAUDE.md`, `plan/HANDOFF.md`,
   `plan/STATE.md`, `plan/decisions.md` and the brief of your type.
2. Read the newest commits (`git log -15`). A worker puts its summary
   and its weak spots in the commit message.
3. If the wiki is not at `~/dev/wiki`, clone it:
   `gh repo clone nowmichaelclark/wiki ~/dev/wiki` (R26).
4. Tell Mikey in a few lines what is done and what you start next.

## The agenda

- `STATE.md` has one short line for each row, in one table for each
  lead. `plan/rows.md` has the full text of each row.
- A lead does the rows of its own table. If it finds work for a
  different lead, it adds a row to that table. This is the handoff
  between the leads.
- A row is one line in `STATE.md`. Keep `STATE.md` below 90 lines.
- `tools/check-plan.py` (the first step of `tools/check.sh`) fails if
  a row is in only one of the two files, if a row waits on a row that
  is gone, or if the gate line is wrong. Run it after each change of
  the agenda.

## Keep the workflow light

Driftloom, Mikey's other project, became slow for three reasons. Do
not let them start here:

- **A status file with no cap.** Driftloom's queue has about 9,000
  words, and each code session reads it before work. Keep `STATE.md`
  below 90 lines. Keep the full row texts in `rows.md`, and delete a
  row when it is done.
- **One value that every change edits in a shared file.** Each
  Driftloom app change edits the same version number. Thus two
  sessions collide, which makes merge commits, and each change starts
  a long CI run. Do not add a value like this. The build name here is
  the commit id (`game-build.yml` writes `BUILD.txt`). Keep it so.
- **One feature cut into many items, each with its own handoff.**
  Each handoff costs a session that reads the plan again. Make one
  row for one feature that a worker can finish in one session. Split
  a row only when each part can be tested or played alone.

## Workers

- Start one worker for each agenda row with the Agent tool. Give it
  the row number, the model of the row, and the rules of CLAUDE.md.
  Tell it to reply with a short summary and **Weak spots**.
- Give each worker its own git worktree under `~/dev/bf-wt/`. Copy
  `tools/.cache` and `game/addons/terrain_3d` into it (`cp -a
  --reflink=auto`). Give each worker a range of row numbers.
- **Two workers at most** run at the same time (Mikey, 2026-11-15).
- **No overlap.** Before you start two workers, write down the systems
  and the files that each row changes. If they share one, run the rows
  one after the other. Each worker changes only its own row in
  `STATE.md` and `rows.md`. The lead merges the rest.
- **Disk:** each worktree builds its own `rust/target/` (3 to 4 GB).
  When a worker ends, remove its worktree (`git worktree remove`).
- A worker does not measure GPU time. The Speed lead does.
- If Mikey's usage limit stops a worker, a `SendMessage` to the agent
  resumes it. Use Sonnet 5.5, high, for an Opus row when the limit is
  low. Fable 5.2 is not available after 2026-11-16.

## Tools (R29)

`tools/README.md` is the toolbox: the tools that exist, how to run
them, and their limits. Each lead and each worker follows its rules:
look there first, make a tool when you do a thing the second time,
write down the faults of a tool, and make it better. Tell each worker
this. In the handoff, name the tools that are new or changed.

## Play and measure

- Start the game from the command line: `-- --bf-test=benchmark`,
  `--bf-test=draw-distance`, `--bf-autoplay=tour`
  (`plan/diagnostics.md`).
- Add `--gpu-index 0` to use the Intel GPU (the speed target, R5).
  Without it, the game uses the NVIDIA GTX 1650.
- Keep the game window in front while a test runs.
- Read the zip in the `playtests/` folder next to the game. Mikey's own
  playtests are in `~/Games/broken-friends-linux/playtests/`.
- Send each new picture to Mikey with SendUserFile.
- **Look before you send.** Read each picture of a worker yourself.
  Say what is wrong in it, not only what is new. A fault in a picture
  becomes a fix now or a row in the Fix table.
- A weak spot that describes a fault (not a limit) is a bug. Give it a
  row in the Fix table in the same session.

## Permissions

Claude Code asks Mikey before it runs a command. Do not change the
permission settings yourself. If many prompts slow the work, tell
Mikey which commands repeat, and let him decide.

## End

Before the context becomes full, or after one big step, write
`plan/HANDOFF.md` again. Name the next lead type in it and in
`STATE.md`, "Cycle". Then tell Mikey to start that lead.

In the commit message of the handoff, write one line "Model check":
the model and the effort of the lead and of each worker, and if each
was enough, too weak, or more than the work needed. The Bird's-eye
lead reads these lines and changes the model table above.
