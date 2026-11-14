# Handoff to the next Brain session

Written: 2026-11-14, at the end of the first Brain session of the 3D
version (a very long claude.ai Project chat). The next Brain session
may run on a different Claude account, with no memory of that chat.
This file and the other plan files hold everything it needs.

## Start here

1. Read `CLAUDE.md` (the rules for all sessions, and the writing style:
   about 80% Simplified Technical English).
2. Read this file, then `STATE.md`, `decisions.md` (R1 to R23), and
   the newest section of `play-notes.md`.
3. Read the specs only when a topic needs them: `demo-1.md`,
   `tech.md`, `bear.md`, `obstacles.md`, `diagnostics.md`,
   `vision.md`, `local-setup.md`, and `research/`.

## The game in short

Broken Friends is a 3D PC game in Godot 4.7.1 and Rust (godot-rust).
The player is an old, classic teddy bear, 0.40 m tall, in a
procedurally generated, hot, smog-filled world. MIT licence, free, no
ads. Demo 1 is one biome (scorched highlands) where the bear walks,
jumps and climbs (`demo-1.md`).

What is built (all on `main`, all tested by the GitHub build):
- The terrain generator (seeded, 2 x 2 km zones) and Terrain3D.
- The bear: a model made in code from separate parts, procedural
  animation (Overgrowth-style layers, springs, aim IK, a stride wheel),
  climbing like Breath of the Wild (grip, stamina, hop, let go),
  gaits for missing limbs, a Jolt ragdoll tumble, seams that tear,
  limbs that come off and stay in the world, sewing.
- Smog, sun, sky, dust; the draw distance follows the smog; the smoke
  edge around the zone (death means a restart at the start point).
- Eight obstacle types (ash crust, rolling stones, gusts, push vents,
  launch vents, stepping stones, updrafts, sinking sand).
- Diagnostics: the playtest recorder (one zip with `report.md`), the
  gamepad dev menu, a Tests page (Benchmark, Draw distance test,
  Autoplay tour), and autoplay on GitHub after each push and each
  night.

## How Mikey works

- Mikey is an English teacher in Thailand, not a game developer. He
  gives the vision, taste and feel. On structural and technical calls
  he asks Claude to decide as an experienced game developer would.
- **R22:** never ask him technical questions. Read the playtest report.
  If data is missing, add a probe or a test.
- **R10:** he does not tune values. He plays the build from GitHub
  (`tools/get-build.sh`) and says what he feels; sessions change the
  defaults.
- **R13:** he plays with a gamepad, sitting down. Everything must work
  on the gamepad.
- He has the $20 Claude plan, and sometimes a limited amount of Fable
  5.2. Recommend Sonnet 5.5 for well-defined work and Opus 5.5 (or
  Fable 5.2 at medium) for hard, open work. Effort: high for new
  systems, medium for tuning.
- From now on he runs Hands sessions (and maybe Brain sessions) in
  Claude Code on his Fedora 45 laptop (`local-setup.md`). One row for
  each session.
- His laptops: the playtest laptop has an NVIDIA GTX 1650 (discrete).
  The Vivobook has Intel integrated graphics: that is the real speed
  target (R5). His disk space is limited.
- He likes short, clear replies. He does not like long lists of
  questions. Ask only about taste, and only a few things.

## Where things stand

The top problem is **long freezes**: 5 s freezes in the first minute
at the start of seed 1, and a freeze of about 950 ms every few
minutes (`play-notes.md`, S-125). S-136 fixes them.

Recommended order:
1. **S-136**, fix the freezes (Opus 5.5, high).
2. **S-137**, Mikey's test on the Vivobook with integrated graphics.
3. The tuning rows from the S-125 playtest, one at a time: S-132
   (obstacles: launch vents miss too often and tear both legs off;
   rolling stones never hit), S-135 (tumble and seams: legs come off
   too easily), S-126, S-130, S-131 (Sonnet 5.5, medium).
4. S-108 (ground scatter) and S-129 (limb-ripping obstacles).
5. S-109, a Brain session: play the full demo, decide what to fix,
   plan demo 2 (caves, `tech.md` and `research/caves-and-sdf.md`).

S-124 (replay) can wait until a bug cannot be understood from a
report. S-134 waits for the first nightly autoplay result.

## Open questions for Mikey (taste only)

- Obstacles (`obstacles.md`): are these the right kinds? Should the
  bear sometimes *want* to lose a limb, for example to escape a trap?
- The visual style is still open (R15 is a budget, not a style).
- The ash crusts are 6 to 9 m wide. Too big for a small bear?

## Lessons from the first Brain session

- Start a new Brain session after each big step. The first one ran far
  too long, and it should have handed off much earlier.
- Several Hands sessions can run at the same time if they change
  different systems. They merge `STATE.md` by hand; this worked.
- Cloud sessions sometimes could not start Godot or could push only to
  their own branch. Local sessions avoid both. `tools/check.sh` does
  the full end-of-row check.
- Each Hands session sends a summary with "Weak spots". Read them: they
  found real problems (for example the crash on quit, F6 missing on
  the gamepad).
