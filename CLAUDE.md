# For Claude Code sessions

This repo is **Broken Friends**. It is a 3D game for PC, made with
Godot 4 and Rust. The player is a teddy bear. The bear explores a
procedurally generated world that is hot and full of smog. We started
the 3D version from zero in November 2026.

The old 2D game and its plans are in `archive/2d/`. The `2d-concept`
branch also keeps the full old state. Use the old files for ideas
only. Do not use their code or their rules.

## Start of every session

1. Read `plan/STATE.md`. "Run S-NNN" means that row of the agenda.
2. Read the brief for the session type of the row
   (`plan/sessions/<type>.md`).
3. Read `plan/tech.md`. Then read the spec that the row names (now,
   `plan/demo-1.md`).
4. Do not edit `plan/MIKEY.md`. It is Mikey's file.

## The most important rules

- **Rust first.** Put the game logic in Rust (`rust/`). This includes
  movement, the camera, world generation, scatter and debug tools.
  Scenes (`.tscn`) and shaders (`.gdshader`) are permitted. Use
  GDScript only for a short test, and delete it in the same session.
- **Tuning values.** Make each tuning value an `#[export]` field with
  a good default. Mikey does not tune values himself (decision R10).
  He plays the build from GitHub and describes what he feels. Claude
  sessions then change the defaults.
- **Scale.** 1 Godot unit is 1 metre. Y is up. The teddy bear is
  0.40 m tall. All other objects have their real-world size.
- **Speed target.** The game must run at 60 frames per second (fps)
  at 1080p on a laptop with integrated graphics. Use the Mobile
  renderer and a 3D render scale of 0.75. Only Mikey's laptops give
  valid speed results. Results from a cloud container are not valid.
- **Seeds.** The same seed must always make the same world. Do not use
  a random source without a seed in generation.
- **Mikey decides.** If the plans do not answer a design question,
  build the simplest version. Then add it to the "Assumed" list in
  `game/README.md` and ask Mikey. Only Mikey writes decisions in
  `plan/decisions.md`.
- **One job per agenda row.** Do only the work of your row. Write
  ideas for later in `plan/inbox.md`, one line for each idea.
- **Mark guesses.** If you did not check a statement about an engine,
  a crate or speed, add `[unverified]` after it.

## Writing style: about 80% Simplified Technical English

Write docs, explanations, session summaries and doc comments about 80%
of the way to ASD-STE100 (Simplified Technical English). The full
specification is very strict. Use its main rules. If a rule makes the
text unclear or unnatural, do not use that rule.

- Keep sentences short. Steps: 20 words at most. Descriptions: 25
  words at most.
- Write one instruction in each sentence. Start an instruction with a
  verb: "Run the tests," not "The tests should be run."
- Use the active voice and the present tense when possible.
- Use simple, common words. Give each word one meaning only. Write
  "use," not "utilize." Write "start," not "initiate."
- Use technical names (clipmap, SDF, MultiMesh) as they are. Explain a
  technical name the first time a doc uses it.
- Keep "the," "a" and "this." Do not write in note-style fragments.
- If a simple verb is possible, do not use a phrasal verb. Write
  "make," not "set up." Write "do," not "carry out."
- Write about one topic in each paragraph. Use 6 sentences at most.
- Use a numbered list for steps that have an order. Use a bullet list
  for other lists.
- Put a warning or a condition before its instruction: "If the build
  fails, read the log."

This style is for prose only. It is not for code or for Mikey's own
files. When you edit an old doc, write the parts that you change in
this style.

## End of every agenda row

1. In `rust/`, run `cargo fmt`, `cargo clippy` and `cargo test`. Fix
   the problems that they show.
2. Open the game without a window:
   `godot --headless --path game --quit-after 300`. Make sure that it
   loads without errors.
3. Update `plan/STATE.md`. Delete your row. Change the rows that your
   row unblocks to **ready**. Add new rows for follow-up work at the
   bottom. Give each new row a model and an effort level.
4. Make one commit for each agenda row: `<Session type>: <what
   changed>`.
5. Push to `main`: `git pull --rebase origin main`, then
   `git push origin HEAD:main`. If a different session changed the
   same rows, keep the rows of both sessions. Then give your new rows
   new numbers.
6. If the push changes `rust/`, `game/` or `tools/`, GitHub builds the
   game (`.github/workflows/game-build.yml`). If you can, check that
   this build passes. If it fails, fix it before you stop.
7. At the end of your summary, write **Weak spots**: three or more
   honest problems that you see.

If a session does more than one agenda row, do all of these steps at
the end of each row, before you start the next row.
