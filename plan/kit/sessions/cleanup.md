# Cleanup session: keep the repo lean

**Tier:** F (a small model is fine). **With:** nobody.

## Purpose

Keep the repo easy to read for the next session. You fix form, never
meaning.

## Pack with

The output of `python3 kit/tools/check.py` pasted into the task, and
the files it complains about. Plus `verify.md`, `open-questions.md`
and `glossary.md`.

## What to do

1. Fix everything `check.py` reported in the files you were given:
   broken links, missing frontmatter, wrong statuses.
2. Over-budget files: propose a split (which sections go to which new
   file) in your reply. Don't do the split unless the task says so.
3. `verify.md`: move ticked items to the log note as a one-line
   summary each, and remove them from the file.
4. `open-questions.md`: remove questions that a decision or a doc now
   answers, naming where the answer lives in the log note.
5. Glossary: list terms used in docs but missing from the glossary,
   and synonyms used for the same thing. Fix the glossary; list the
   doc fixes for a Drafter.
6. Folder warnings (RULES section 6): for each `inbox/` or
   `research/` file, put every idea not yet in a doc or decision as one
   line under the right system's seed notes (citing the file's date),
   then delete the raw file. Delete the oldest `log/` notes down to
   five, and `reviews/` whose fixes are all made. Say in the log note
   what went where. Warnings about `game/` or `rust/` files are for the
   next Build session: leave them.
7. Never change what a doc means. If something looks wrong, put it in
   **Weak spots** instead.

Return the files, the log note and **Weak spots**.
