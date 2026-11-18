#!/usr/bin/env python3
"""Report the cost of the workflow, one line for each week (S-231, R29).

The Bird's-eye lead runs it (plan/sessions/lead-birdseye.md). It needs
no packages, only git. It reads the history of the current branch:

  tools/workflow-cost.py            the last 8 weeks
  tools/workflow-cost.py --weeks 4  the last 4 weeks

The columns:

- Docs only: the commits that change only docs. A file is a doc if its
  name ends in ".md", or if it is in plan/ or archive/. Merge commits
  are not counted.
- Read before work: the words that a lead reads before it starts:
  CLAUDE.md, plan/HANDOFF.md, plan/STATE.md, plan/decisions.md,
  plan/tech.md and the longest brief in plan/sessions/.
- STATE.md lines (the cap is 90) and rows.md words.

The sizes are those of the last commit of the week. A week is Monday
to Sunday, by commit date. If a number climbs two weeks in a row, the
report says so. Then the Bird's-eye lead adds a Hygiene row.
"""
import argparse
import datetime as dt
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
READ_FIRST = [
    "CLAUDE.md",
    "plan/HANDOFF.md",
    "plan/STATE.md",
    "plan/decisions.md",
    "plan/tech.md",
]
BRIEFS = "plan/sessions/"
STATE_CAP = 90  # the same as MAX_LINES in tools/check-plan.py


def git(*args):
    out = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=True
    )
    return out.stdout


def show(commit, path):
    """The text of a file at a commit, or "" if the file is not there."""
    try:
        return git("show", f"{commit}:{path}")
    except subprocess.CalledProcessError:
        return ""


def is_doc(path):
    return path.endswith(".md") or path.startswith(("plan/", "archive/"))


def week_of(day):
    """The Monday of the week of a date."""
    return day - dt.timedelta(days=day.weekday())


def sizes(commit):
    """Words before work, the longest brief, STATE.md lines, rows.md words."""
    words = sum(len(show(commit, p).split()) for p in READ_FIRST)
    briefs = [
        p for p in git("ls-tree", "--name-only", commit, BRIEFS).split() if p
    ]
    longest = max(briefs, key=lambda p: len(show(commit, p).split()), default="")
    words += len(show(commit, longest).split()) if longest else 0
    state = len(show(commit, "plan/STATE.md").splitlines())
    rows = len(show(commit, "plan/rows.md").split())
    return words, longest, state, rows


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--weeks", type=int, default=8, help="weeks to show")
    args = ap.parse_args()

    if git("rev-parse", "--is-shallow-repository").strip() == "true":
        print(
            "This clone is shallow, so old weeks are missing. "
            "Run: git fetch --unshallow",
            file=sys.stderr,
        )

    # Count the commits of each week (merges are not counted).
    count, docs = {}, {}
    log = git("log", "--no-merges", "--format=@%H %cs", "--name-only")
    for block in log.split("@")[1:]:
        lines = block.split("\n")
        _, day = lines[0].split()
        files = [f for f in lines[1:] if f]
        week = week_of(dt.date.fromisoformat(day))
        count[week] = count.get(week, 0) + 1
        if files and all(is_doc(f) for f in files):
            docs[week] = docs.get(week, 0) + 1

    # The last commit of each week on the main line.
    last = {}
    for line in git("log", "--first-parent", "--format=%H %cs").splitlines():
        commit, day = line.split()
        last.setdefault(week_of(dt.date.fromisoformat(day)), commit)

    # Every week from the first commit to today. A week with no commit
    # keeps the sizes of the week before it.
    today = week_of(dt.date.today())
    week, weeks = min(set(count) | set(last)), []
    while week <= max(today, *last):
        weeks.append(week)
        week += dt.timedelta(days=7)
    table, commit = [], None
    for week in weeks:
        commit = last.get(week, commit)
        if week not in weeks[-args.weeks:]:
            continue
        words, longest, state, rows = sizes(commit) if commit else (0, "", 0, 0)
        n, d = count.get(week, 0), docs.get(week, 0)
        share = round(100 * d / n) if n else None  # no commits: no share
        table.append((week, n, d, share, words, longest, state, rows))

    print("| Week (Monday) | Commits | Docs only | Read before work | STATE.md lines | rows.md words |")
    print("|---|---|---|---|---|---|")
    for week, n, d, share, words, _, state, rows in table:
        mark = " (so far)" if week == today else ""
        cell = f"{d} ({share}%)" if share is not None else "–"
        print(
            f"| {week}{mark} | {n} | {cell} | {words:,} words "
            f"| {state} of {STATE_CAP} | {rows:,} |"
        )
    if table:
        print(f"\nThe longest brief now: {table[-1][5] or '(none)'}.")

    # A number that climbs two weeks in a row needs a Hygiene row.
    names = {3: "Docs-only share", 4: "Read before work", 6: "STATE.md lines", 7: "rows.md words"}
    climbs = [
        name
        for i, name in names.items()
        if len(table) >= 3
        and None not in (table[-3][i], table[-2][i], table[-1][i])
        and table[-3][i] < table[-2][i] < table[-1][i]
    ]
    if climbs:
        print("Climbs two weeks in a row: " + ", ".join(climbs) + ". Add a Hygiene row.")
    else:
        print("No number climbs two weeks in a row.")


if __name__ == "__main__":
    main()
