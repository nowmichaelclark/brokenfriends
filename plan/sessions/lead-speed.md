# Speed lead: measure and cut, on a quiet laptop

Read `lead.md` first. The Speed lead keeps the game at 60 fps at 1080p
on the Intel GPU (R5, R15). It does the rows of the "Speed" table in
`STATE.md`. It runs alone: no other lead and no feature worker.

## Steps

1. Download the newest GitHub build (`tools/get-build.sh`). Measure
   the build that Mikey plays, not a local debug build.
2. Run the tests with `--gpu-index 0`: Benchmark, Water view, Cave
   tour, autoplay tour. Use one new process for each setting.
3. Compare each system with its budget (`demo-2.md`, `tech.md`). Write
   the numbers in `play-notes.md`, newest at the top.
4. For each system above its budget, do its row. Do the cut yourself,
   or start one worker and wait for it. Do not measure while it builds.
5. Measure again after each cut, in the same hour as the "before" run.

## Rules

- The noise of one run is about 0.5 ms, and it changes with the hour.
  Run each setting 3 times, and compare only A and B of the same hour.
- 60 fps is 16.7 ms for the whole frame. A view above 16.7 ms is a
  problem even if each system is inside its budget.
- Keep the look. If a cut changes the look, send Mikey a before and an
  after picture, and add one taste question to the next play row.
- If a number is missing, add a probe or a test (R22). Do not guess.
- Mark each statement that you did not measure with `[unverified]`.

## End

Write the result table in `play-notes.md` and `HANDOFF.md`. The next
lead is usually Hygiene, or Mikey's play.
