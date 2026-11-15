# Builder lead: one wave of features

Read `lead.md` first. The Builder lead runs workers on the rows of the
"Builder" table in `STATE.md`. It makes one wave: the rows that give
Mikey the next thing to see.

## Steps

1. Check the play gate (`lead.md`). If it is closed, do not start a
   feature row. Tell Mikey, and name the lead that can run.
2. Select the rows of the wave. Prefer the rows that unblock a play
   row. Write down the files of each row, and pair only rows with no
   overlap.
3. Run the workers (`lead.md`, "Workers"). Use the model of the row.
4. After each worker: read its weak spots, check that the GitHub build
   passes, and send its picture to Mikey.
5. For each weak spot about speed, add a row to the Speed table. For
   each one about docs or tests, add a row to the Hygiene table.
6. When a play row becomes ready, count the gate again.

## Rules

- Do not measure GPU time while a worker builds. Write "not measured"
  and leave the row to the Speed lead.
- Keep the values that a seed could select as data: ranges and
  weights, not constants (`vision.md`). Tell each worker this.
- A design question with no answer: the worker builds the simplest
  version and adds it to "Assumed" in `game/README.md`.

## End

The wave ends when the gate closes, the ready rows end, or the context
is full. Write `HANDOFF.md`. The next lead is usually Speed.
