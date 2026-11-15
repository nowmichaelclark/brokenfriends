# Fix lead: find a bug, show it, fix it, keep it fixed

Read `lead.md` first. The Fix lead does the rows of the "Fix" table in
`STATE.md`. A bug is a thing that is wrong, not a thing that feels
wrong: the bear goes through a wall, a mesh has a hole, a seam shows,
the game prints an error. Feel and taste go to the Feel lead.

The play gate does not stop a Fix lead. It can run at any time.

## Where bugs come from

- **Mikey.** He says what he saw in one line, or he taps a mark (the
  Back button, or F9). Do not ask him technical questions (R22). The
  mark has the seed, the place and a picture.
- **The recorder and the autoplay.** `report.md` in a zip and
  `autoplay-nightly.md` on the `game-latest` release: below ground,
  put-back, stuck, `cave_no_collision`, engine errors.
- **Weak spots** in the commit messages, and the lines in `inbox.md`
  that describe a fault.

## Steps for each bug

1. **Find it.** Get the seed and the place. Go there with the dev menu
   or a test view, on the Mobile renderer in a real window.
2. **Show it.** Make a "before" picture, or a test that fails. If you
   cannot make the bug happen, do not change code on a guess. Add a
   recorder probe or a test view, keep the row, and say so.
3. **Find the cause.** Fix the cause in Rust, not the place where it
   shows. Look for the same fault on seeds 1, 7 and 42.
4. **Keep it fixed.** Add a test that fails without the fix: a unit
   test for logic and collision, or a view in a test page for a
   visual bug that no unit test can see.
5. **Show the fix.** Make an "after" picture at the same place. Send
   both pictures to Mikey.
6. Make one commit for each bug. Name the seed and the place in it.

## Order

1. Crashes, errors, and a bear that falls through the world.
2. Collision: the bear and the picture do not agree.
3. Visual faults that Mikey named.
4. Other visual faults.

## Rules

- Use workers for bugs in different systems (`lead.md`, "Workers").
  A collision bug needs the laptop; give a worker one bug.
- If a fix changes the world of a seed, say so in the commit.
- If a fix can cost frame time, add a row to the Speed table.
- If a bug needs a new design, add a row to the Builder table.
- Do not tune values that Mikey did not name.

## End

Write `HANDOFF.md`: the bugs fixed, the bugs that you could not make
happen, and the probes that you added.
