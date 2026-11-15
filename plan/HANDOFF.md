# Handoff to the next lead Brain session

Written: 2026-11-15, evening, at the end of the first lead Brain
session on Mikey's laptop (Claude Code, Opus 5.5). The next session
starts with `plan/sessions/lead.md`. This file and the other plan
files hold everything it needs.

## Start here

1. Read `CLAUDE.md`, then `plan/sessions/lead.md` (how the lead runs
   workers), then this file.
2. Read `STATE.md`, `decisions.md` (R1 to R26) and the two newest
   sections of `play-notes.md` (the Intel GPU numbers).
3. Read `demo-2.md` when you start a demo 2 row.

## The game in short

Broken Friends is a 3D PC game in Godot 4.7.1 and Rust (godot-rust).
The player is an old teddy bear, 0.40 m tall, in a procedural, hot,
smoggy world. Demo 1 (one biome: walk, jump, climb, obstacles, tumble,
seams, smoke edge) is built and waits for Mikey's feel notes. Demo 2
(water and caves, R24, R25) started on 2026-11-15.

## What this session did

- Ran the S-137 tests on the Intel GPU itself. Mikey only needs to
  play 10 minutes for S-137.
- Read the cloud results of S-132, S-135 and S-142 (the demo 2 plan).
- Ran workers in git worktrees under `~/dev/bf-wt/` (all removed now):
  S-161 caves layer, S-160 still water, S-180 cheaper water, S-164
  walkable caves, S-162 rivers, S-166 cave light, S-165 waterfalls.
  All are on `main`, and all GitHub builds pass.
- Measured all of it on the Intel GPU (`play-notes.md`). Short form:
  - Lakes 1.1 to 2.2 ms and rivers 1.1 to 1.8 ms (budget 1.0 ms).
  - Waterfalls 0.75 ms at most (budget 0.8 ms).
  - Caves with the deep-cave cut: 71 to 73 fps. The cut saves 5 ms.
  - Views of lakes and cave mouths need 15 to 18 ms even without
    water. This base cost is the next speed problem (S-183).

## Recommended order

1. **S-163** (swimming, Sonnet, high). It is the last row before
   Mikey's first look at demo 2 (S-167). Parallel with it: **S-187**
   (dev menu caves line and "go to the nearest mouth", Sonnet, low),
   so that Mikey can find a cave. Check the overlap first: both may
   touch `bear.rs`.
2. Then tell Mikey to play **S-167** (and S-137 and S-184 in the same
   sitting). His answers unblock S-191, S-192, S-202 and the tuning.
3. Speed rows, one at a time with a quiet laptop: S-183 (base cost of
   open views), S-181 (water to 1.0 ms), S-205 (cave base cost).
4. Look rows: S-186 (cave rim), S-201 (falls look), S-195 then S-168
   (light on water, then pools in caves), S-169 (far walls, Opus).

## Lessons for the lead

- Two workers at most. Give each its own worktree. Copy
  `tools/.cache` and `game/addons/terrain_3d` into it (`cp -a
  --reflink=auto`), so `tools/check.sh` downloads nothing.
- Workers must not measure GPU time when another build runs. The lead
  measures after the builds, with the GitHub build
  (`tools/get-build.sh`), one process for each setting.
- Mikey's usage limit stopped two Opus workers once. A `SendMessage` to
  the stopped agent resumed it with no lost work. Use Sonnet, high, for
  Opus rows when the limit is low (STATE.md, "How we chose the models").
- The workers write good weak spots. Read them before the next row.
- Workers send a screenshot path. Send the picture to Mikey with
  SendUserFile. He likes to see each new thing.

## Open questions for Mikey (taste only)

- The three demo 2 questions in `demo-2.md`: water colours, light in
  deep caves, thick water as a danger.
- Longer rivers (S-191) and true cliffs for the big falls (S-202).
- The new water look after S-180 (S-184).
- Demo 1: are the obstacles the right kinds? The visual style.
