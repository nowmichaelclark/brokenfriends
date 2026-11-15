# Handoff to the next lead session

Written: 2026-11-15, night, at the end of S-206 (a planning lead,
Fable 5.2). **The next lead is a Fix lead**, for the bug tools S-213
to S-216 (not for the bugs first), then a Builder lead. Mikey starts it with: "Start the Fix lead. Read
plan/sessions/lead.md."

## Start here

1. Read `CLAUDE.md`, then `plan/sessions/lead.md` (the cycle, the play
   gate, the workers), then `plan/sessions/lead-builder.md`.
2. Read `STATE.md`. It has one line for each row. The full text of a
   row is in `rows.md`.
3. Read `decisions.md` (R1 to R28) and `demo-2.md`.

## The game in short

Broken Friends is a 3D PC game in Godot 4.7.1 and Rust (godot-rust).
The player is an old teddy bear, 0.40 m tall, in a procedural, hot,
smoggy world. Demo 1 is built and waits for Mikey's feel notes. Demo 2
(water and caves) is in work: lakes, rivers, falls, caves and the cave
light are on `main` (build 2e2d5f8).

## What S-206 changed

- **R27:** seven types of lead (Builder, Speed, Fix, Feel, Hygiene,
  Research, Bird's-eye), one at a time, in a cycle. Each has a brief
  in `plan/sessions/lead-<type>.md`. The Fix lead is for bugs, and it
  can run at any time.
- **R28:** the play gate. If two play rows are ready, the Builder
  starts only rows with "–" or a ready play row in the "Play" column.
- `tools/check-plan.py` (the first step of `tools/check.sh`) checks
  that `STATE.md` and `rows.md` agree and that the gate line is true.
- The briefs `build.md`, `brain.md` and `play.md` agree with R26 to
  R28.
- `STATE.md` has one table for each lead and short rows. `rows.md` is
  new and has the full row texts.
- S-184 is now part of S-167, so Mikey plays one sitting.
- New rows: S-207 and S-208 (Hygiene), S-209 and S-210 (Research),
  S-211 (Bird's-eye), S-212 (Fix: the bugs that Mikey sees now).

## Mikey's bugs (S-212)

Mikey said on 2026-11-15 that the game has visual bugs and collision
bugs now. His first three are in `rows.md`, S-212: the water edge is
square and does not follow the ground, a bright haze shows deep in the
caves, and the camera sometimes goes through the ground. He says there
are more. Do not ask him for seeds or numbers (R22): one line of
words, or a mark, for each bug.

Mikey does not want these bugs fixed first. He wants a way of working
that finds such bugs without him. `diagnostics.md`, "Finding bugs",
has the plan: numbers (world audit, play probes), pictures (look tour,
look review), then Mikey. The three bugs are the test: a tool must
show each one before a session fixes it. Evidence from S-206: the
Cave tour pictures of 18:27 already show bright blobs and haze in the
chambers, behind the debug overlay. No session looked at them for
faults. The water bug and S-181 change the
same files, so do not run them at the same time.

## The first wave

1. **S-163** (swimming, Sonnet 5.5, high). Beside it, one after the
   other: **S-187** and **S-186** (their play row is S-167 too). Check
   the files first: S-187 and S-163 can both change `bear.rs`.
2. When S-163 is on `main`, S-167 is ready and the gate closes. Tell
   Mikey to play S-137 and S-167 in one sitting.
3. Then a Speed lead (S-182, S-183, S-181, S-205), a Fix lead (S-134,
   S-212) and a Hygiene lead. A Research lead (S-209) can use a wait.
4. Write a "Model check" line in the commit of your handoff
   (`lead.md`, "End").

## Lessons for a lead

- Copy `tools/.cache` and `game/addons/terrain_3d` into each worktree,
  so `tools/check.sh` downloads nothing.
- Only the Speed lead measures, with the GitHub build and one process
  for each setting.
- The usage limit stopped two Opus workers once. A `SendMessage`
  resumed them with no lost work.
- Mikey likes to see each new thing. Send the pictures.

## Open questions for Mikey (taste only)

- The three demo 2 questions in `demo-2.md`: water colours, light in
  deep caves, thick water as a danger.
- Longer rivers (S-191) and true cliffs for the big falls (S-202).
- The new water look after S-180 (in S-167).
- Demo 1: are the obstacles the right kinds? The visual style.
