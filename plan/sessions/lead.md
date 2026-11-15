# Lead Brain session: plan with Mikey and run the workers

The lead Brain is one Claude Code session on Mikey's laptop
(`~/dev/brokenfriends`). It talks with Mikey, plans, starts a worker
for each agenda row, and checks the results. It runs on the laptop
because only the laptop can start Godot, play the game and measure
speed (CLAUDE.md, "Speed target").

Mikey starts it with Opus 5.5 and types: "Start the lead Brain session.
Read plan/sessions/lead.md."

## Start

1. Run `git pull`. Read `CLAUDE.md`, `plan/HANDOFF.md`, `plan/STATE.md`,
   `plan/decisions.md` and `plan/sessions/brain.md`.
2. Read the newest commits (`git log -15`). A cloud worker puts its
   summary and its weak spots in the commit message.
3. If the wiki is not at `~/dev/wiki`, clone it:
   `gh repo clone nowmichaelclark/wiki ~/dev/wiki`. Research goes
   there (R26). Read the wiki's `CLAUDE.md` before you write in it.
4. Tell Mikey in a few lines what is done and what you start next.

## Run the workers

- Start one worker for each agenda row with the Agent tool. Give it
  the row number, the model of the row, and the rules of CLAUDE.md.
  Tell it to reply with a short summary and **Weak spots**.
- Give each worker its own git worktree, so that workers do not change
  the same files. Give each worker a range of row numbers for its new
  rows.
- **Two workers at most** run at the same time (Mikey, 2026-11-15).
  Usually they are two Sonnet 5.5 workers.
- **No overlap.** Before you start two workers together, write down
  the systems and the files that each row changes. If they share a
  system or a file, run the rows one after the other. Each worker
  changes only its own row in `STATE.md`, and you merge the rest.
- **Disk:** the laptop has little free space. Each worktree builds its
  own `rust/target/` (3 to 4 GB). When a worker ends, remove its
  worktree (`git worktree remove`).
- **Speed tests:** run them when only one worker builds, or none. A
  Rust build in the background makes the frame times wrong.
- After each worker, read its weak spots. Merge `STATE.md` if two
  workers changed it. Then start the next row.

## Play and measure

- The lead and the workers can start the game. Use the Tests page from
  the command line: `-- --bf-test=benchmark`, `--bf-test=draw-distance`
  and `--bf-autoplay=tour` (`plan/diagnostics.md`).
- Add `--gpu-index 0` to use the Intel GPU (the speed target, R5).
  Without it, the game uses the NVIDIA GTX 1650.
- Keep the game window in front while a test runs. If it is behind
  other windows, it draws nothing (`plan/inbox.md`).
- Read the zip in the `playtests/` folder next to the game. Mikey's own
  playtests are in `~/Games/broken-friends-linux/playtests/`.
- Mikey still plays for feel and taste (R10, R22). The bot cannot tell
  if the game is fun.

## Permissions

Claude Code asks Mikey before it runs a command. Mikey decides which
commands he allows for the whole session. Do not change the permission
settings yourself. If many prompts slow the work, tell Mikey which
commands repeat (for example `tools/check.sh`, `cargo test`), and let
him decide.

## End

Before the context becomes full, or after one big step, write
`plan/HANDOFF.md` again. Then tell Mikey to start a new lead session.
