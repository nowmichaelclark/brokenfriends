# Standing rules

These apply to every session on every model. `START-HERE.md` has the
short version; this is the full one. If a rule and a task conflict,
stop and say so rather than picking one.

## 1. One job per session

- Do the task in your pack. Edit only the files it names, plus
  `glossary.md`, `open-questions.md` and `verify.md` when the rules
  below call for it.
- Small sessions beat long ones. If a task would need more than about
  three docs, say so and suggest how to split it.
- **The agenda** (`STATE.md`) lists every planned session: its number
  (S-NNN), type, model and effort, and status: **ready**, or waiting on
  what. Mikey starts one with "Run S-NNN". When yours ends, delete its
  row, flip the rows it unblocks to ready, and add follow-ups (with the
  model and effort you'd recommend) at the bottom. Touch only those
  rows, so sessions running in tandem don't clash.
- **The rhythm.** A Cleanup session at least every three other
  sessions (`check.py` counts, by commit). A Red team after each
  prototype is played, before anything is built on it. A gate review
  at the end of each phase (`kit/PHASES.md`).

## 2. Output format

When you return files, put **all of them inside one code block**
that opens and closes with a line of four tildes (`~~~~`). Inside it,
each **whole file** sits between marker lines, like this:

```
    ~~~~
    === FILE: systems/friend-body/README.md ===
    (the complete file, raw markdown, from first line to last)
    === END FILE ===

    === FILE: log/2026-10-24-brain-pitch.md ===
    (the complete file)
    === END FILE ===
    ~~~~
```

(Shown indented here; in your reply every line starts at the very
beginning of the line.)

Why: Mikey copies your reply from a chat window. Text inside a code
block keeps its raw markdown (`#` headings, `-` bullets, `---`
frontmatter); text outside it can lose that when copied.

- Never return a partial file, a diff, or "the rest stays the same".
  The unpack tool overwrites the file with what you send.
- Don't put other code fences around individual files.
- If a file you should update isn't in your pack, don't return it.
  List the lines to add, after the code block, and say why.
- Paths are relative to the `plan/` folder (so `systems/...`, not
  `plan/systems/...`).
- Write your conversation and **Weak spots** outside the file blocks.
- During a back-and-forth, don't return files until Mikey asks or the
  task is finished. Talk first, write last.

## 3. Facts versus design

Two kinds of statements live in these docs, and they are treated
differently.

- **Design** (what the game should be: rules, feel, story, numbers to
  try) belongs to Mikey. Models offer options, never verdicts.
- **Facts** (what Godot, a plugin, a platform or a device can do, how
  fast something runs, what a tool supports) must be verified.

Every factual claim you haven't checked against a real source in this
session gets `[unverified]` right after it, and one line in `verify.md`:

```
- [ ] V-012: Godot 2D joints do not break on their own. (systems/friend-body) Confirm in: Godot class reference for Joint2D.
```

If you did check it, cite the source in the doc's **Sources** section
instead. The Verifier clears `[unverified]` tags in batches (see
`kit/sessions/verifier.md`). A doc with any `[unverified]` tag left
cannot be `locked`; `check.py` enforces this.

## 4. Decisions

- A decision exists only when Mikey has made it. Record it with
  `kit/templates/decision.md` as `decisions/NNNN-short-name.md`,
  signed `Mikey, YYYY-MM-DD`.
- Models may write a decision file with status `proposed` to set out
  options. Only Mikey changes it to `accepted` or `rejected`.
- To change an accepted decision, write a new one that supersedes it.
  Set the old one's status to `superseded` and `superseded_by` to the
  new number. Never rewrite an accepted decision's reasoning.

## 5. Replace, don't append

- Edit docs in place so they always say what is true **now**. Git keeps
  the history.
- `STATE.md` is rewritten at the end of a session whenever the phase,
  the next steps or the blockers change.
- Resolved open questions are removed from `open-questions.md` once
  they become decisions or doc text.
- Ticked items in `verify.md` are cleared by the Cleanup session.

## 6. Keeping the repo lean

Every line a session must read costs time and attention, and long
files get skimmed. So:

- **Read on demand.** Every session reads only the hot set
  (`START-HERE.md`, this file, `STATE.md`, `CLAUDE.md`; 450 lines in
  all [unverified, V-014]), then follows links to what its task
  needs. Never read a folder whole.
- **One fact, one home.** A fact lives in one doc; others link to it
  or cite the decision number instead of restating it.
- **Digest, then delete.** Raw material (`inbox/`, `research/`, old
  `log/` notes, `reviews/` whose fixes are made) is temporary. Once its
  ideas live in the docs they belong to (a line each, under the right
  system's seed notes), delete the raw file.
- **Git is the archive.** No `old/` or `archive/` folders, no copies of
  past versions, no commented-out code. To find something deleted:
  `git log --all -- <path>`.
- **Code too.** `game/README.md` says what the build is and assumes
  now, not a history of steps (220 lines). A script over 600 lines
  [unverified, V-015] is split before more is added. Sketch code
  that's replaced is deleted, not kept beside the new version.
- **Nothing generated is committed**: screenshots, builds, packs.

`check.py` warns when a folder holds too many files [unverified,
V-016], the hot set or a script is too long, or a doc is over its
budget below. A warning is a job for the next Cleanup session
(`kit/sessions/cleanup.md`) or, for code, the next Build session. Over
budget means split or cut; a limit changes only on evidence, in a
Verifier session, never to make a file fit.

| File | Lines |
|---|---|
| `START-HERE.md` | 120 |
| `STATE.md` | 100 |
| `00-vision/*.md` | 150 each |
| `systems/*/*.md` | 300 each (split a big system into sub-docs) |
| `systems-map.md` | 250 |
| `glossary.md` | 300 |
| `decisions/*.md` | 80 each |
| `open-questions.md` | 200 |
| `verify.md` | 200 |
| `reviews/*.md`, `research/*.md` | 200 each |
| `prototypes/*.md` | 150 each |
| `log/*.md` | 40 each |

## 7. Words

- Use glossary terms exactly as defined. Don't invent synonyms
  ("stuffing" and "fluff" can't both mean the same thing).
- A new term gets an entry in `glossary.md` in the same session.
- Write plainly: short sentences, everyday words, jargon explained the
  first time.
- American English spelling (color, behavior, favorite).

## 8. Status ladder

| Status | Meaning | Who sets it |
|---|---|---|
| `idea` | A stub or rough notes | anyone |
| `draft` | Every template section filled or marked "none yet" | any session |
| `review` | Draft meets the rubric; waiting on a red team | any session |
| `locked` | Mikey approved it; build plans may rely on it | **Mikey only** |
| `prototyped` | A prototype tested it; doc updated with what we learned | Mikey |

A locked doc changes only through a decision that reopens it to
`draft`. If your task would change a locked doc, stop and say so.

## 9. Honesty

- End every reply with **Weak spots**: at least three real problems,
  risks or unknowns in what you produced or what you read. "None" is
  not an answer.
- Disagree with Mikey when you think he's wrong, and say why. Agreeing
  to be pleasant wastes his planning.
- If you don't know, say so. A guess presented as fact is the worst
  thing a planning session can produce.

## 10. Closing a session

Before you finish, return:

1. The files you changed.
2. A log note, `log/YYYY-MM-DD-<session>-<topic>.md`, from
   `kit/templates/session-log.md` (under 40 lines).
3. `STATE.md`, rewritten, if the phase, next steps or blockers changed.

## 11. Stop rather than guess

Stop and report instead of carrying on when:

- the task conflicts with a locked doc or an accepted decision;
- you'd need a file that isn't in your pack;
- a design call is needed that Mikey hasn't made.

Write what you need as a question, in your reply or in
`open-questions.md`.
