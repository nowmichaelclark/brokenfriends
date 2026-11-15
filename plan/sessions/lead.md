# Lead sessions: the cycle and the common rules

A lead is one Claude Code session on Mikey's laptop
(`~/dev/brokenfriends`). It runs on the laptop because only the laptop
can start Godot, play the game and measure speed (CLAUDE.md, "Speed
target"). There are six types of lead. Only one lead runs at a time
(R27).

Mikey starts a lead with: "Start the <type> lead. Read
plan/sessions/lead.md." If he gives no type, use the type that
`STATE.md` names in "Cycle".

## The six leads

| Lead | Brief | Job | Model, effort |
|---|---|---|---|
| Builder | `lead-builder.md` | Runs workers on feature rows | Opus 5.5, medium |
| Speed | `lead-speed.md` | Measures and cuts frame time, alone | Sonnet 5.5, high |
| Feel | `lead-feel.md` | Turns Mikey's play into new defaults | Sonnet 5.5, medium |
| Hygiene | `lead-hygiene.md` | Docs, tests, CI, weak spots | Sonnet 5.5, medium |
| Research | `lead-research.md` | Wiki articles, research for a spec | Sonnet 5.5, medium |
| Bird's-eye | `lead-birdseye.md` | Sorts, checks the direction, plans | Fable 5.2 or Opus 5.5 |

## The cycle (R27)

1. **Builder:** one wave of feature rows, until the play gate closes
   or the ready rows end.
2. **Speed:** measure the wave on the Intel GPU. Cut what is above
   its budget.
3. **Hygiene:** clean after the wave.
4. **Mikey plays** all ready play rows in one sitting.
5. **Feel:** read the zip and the notes, change the defaults, give
   Mikey a new build.
6. **Bird's-eye:** every week, or after a play that changes the plan.

The Research lead has no fixed place. Run it before a new spec, or
while the cycle waits for Mikey. A lead can stop early. Then it writes
in `STATE.md`, "Cycle", which lead is next.

## The play gate (R28)

Count the play rows with the status **ready**. If two or more are
ready, the gate is closed. Then the Builder lead does not start a
feature row. The Speed, Hygiene, Research and Feel leads can run. The
gate opens when Mikey has played. Write the gate status in `STATE.md`,
"Cycle".

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
  low. Use Fable 5.2 only for the Bird's-eye lead and the hardest rows.

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

## Permissions

Claude Code asks Mikey before it runs a command. Do not change the
permission settings yourself. If many prompts slow the work, tell
Mikey which commands repeat, and let him decide.

## End

Before the context becomes full, or after one big step, write
`plan/HANDOFF.md` again. Name the next lead type in it and in
`STATE.md`, "Cycle". Then tell Mikey to start that lead.
