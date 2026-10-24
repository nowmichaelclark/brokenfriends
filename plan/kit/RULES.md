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

## 2. Output format

When you return files, put **all of them inside one code block**
that opens and closes with a line of four tildes (`~~~~`). Inside it,
each **whole file** sits between marker lines, like this:

```
    ~~~~
    === FILE: systems/creature-body/README.md ===
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
- [ ] V-012: Godot 2D joints do not break on their own. (systems/creature-body) Confirm in: Godot class reference for Joint2D.
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

## 6. Size budgets

Long files get skimmed, and cheap models lose track of them. `check.py`
warns at these limits (lines). Over budget means split the doc or cut
it, not raise the limit.

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
