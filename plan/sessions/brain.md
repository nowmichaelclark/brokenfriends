# Brain session: think with Mikey

A Brain session plans with Mikey. It runs in Claude Code in the repo
(`plan/local-setup.md`), so it can read and change the plan files and
push them. If it runs in a chat that cannot write to the repo, it
gives each file change as a full block, and the next Hands session
applies it.

## Read first

`HANDOFF.md` (the newest handoff), `STATE.md`, `decisions.md`, the
newest section of `play-notes.md`, and the spec that the row names.

## How to run it

- Give two or three options with their trade-offs, and say which one
  you recommend. Mikey selects. On structural calls he often asks
  Claude to decide as an experienced game developer would.
- Write a decision in `decisions.md` only after Mikey agrees. Add his
  name and the date.
- Do not ask Mikey technical questions (R22). Read the playtest
  reports. If data is missing, add a recorder probe or a test as a new
  row.
- At the end, rewrite the agenda in `STATE.md`. For each row, give the
  place, the model and the effort that you recommend.
- At the end of your reply, write **Weak spots**: three or more honest
  problems.

## Keep the context small

Start a new Brain session after one big step (for example after a
playtest is read and the next rows are planned). A long Brain session
costs more usage and loses detail. Before it ends, a Brain session
writes `HANDOFF.md` again for the next one.
