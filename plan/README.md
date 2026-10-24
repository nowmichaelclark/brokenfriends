# Broken Friends: the plan

This folder holds the **plans** for Broken Friends, a 2D, mobile-first
creature-care game: stuffed creatures you feed, soak, rip and sew,
train with minigames, and send out on delivery runs. Game code goes in
`../game/` once the planning gates say so (see `kit/PHASES.md`).

Most planning happens on free or cheap AI models. The kit makes that
safe: small sessions, strict output formats, every technical claim
tagged until verified, a written definition of done, and a few
strong-model reviews at the gates.

## The loop, in four commands

Run these from inside `plan/` (`cd ~/projects/brokenfriends/plan`).

```bash
# 1. Build a session pack: the brief, the rules and only the files needed
python3 kit/tools/pack.py drafter systems/creature-body/README.md \
    --task "Turn my notes into a full draft of this system doc." --copy

# 2. Paste the pack into any chat model. Talk it through. When the model
#    returns finished files, save its reply (or copy it) and unpack it:
wl-paste | python3 kit/tools/unpack.py -

# 3. Check the rules: size budgets, statuses, unverified claims, links
python3 kit/tools/check.py

# 4. Look at what changed, then commit
git diff
git add -A && git commit -m "Drafter: creature-body first draft"
```

`--copy` and `wl-paste` use your clipboard (on Fedora KDE, `wl-clipboard`:
`sudo dnf install wl-clipboard`). Without them, use `--out pack.md` and
`unpack.py reply.txt`.

Using DeepSeek (or any chat model) in a browser: the Python runs on your
laptop, before and after the chat. Start a new chat for each session.
When the model returns files, they sit inside one code block (fenced
with `~~~~`). Copy them with **that code block's copy button**, not by
selecting the page text: selected text loses its `#` headings and `-`
bullets, and `unpack.py` refuses it. If the model forgets the code
block, ask: "Put all the files in one ~~~~ code block, as kit/RULES.md
section 2 says."

In Claude Code, sessions edit files directly instead of using packs;
`CLAUDE.md` at the repo root points them at the same rules.

## Where things are

| Path | What |
|---|---|
| `START-HERE.md` | The first thing every AI session reads |
| `STATE.md` | Where planning stands right now (rewritten, never appended) |
| `MIKEY.md` | Mikey's own notes. AI reads, never edits |
| `kit/` | How planning works: rules, rubric, phases, briefs, templates, tools |
| `00-vision/` | Pitch, pillars, non-goals, references, first playable |
| `systems/` | One folder per game system |
| `systems-map.md` | What flows between systems |
| `glossary.md` | Exact meaning of every term |
| `decisions/` | One short file per decision |
| `open-questions.md` | Questions waiting on Mikey |
| `verify.md` | Technical claims waiting to be checked |
| `reviews/` | Red-team and gate reviews |
| `research/` | Notes on reference games and techniques |
| `prototypes/` | Each prototype: the question, the result |
| `inbox/` | Raw ideas, not yet sorted into docs |
| `log/` | One short note per session |
