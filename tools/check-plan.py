#!/usr/bin/env python3
"""Check that plan/STATE.md and plan/rows.md agree (R27, R28).

tools/check.sh runs it first. It needs no packages. It exits with an
error and one line for each problem.
"""
import re
import sys
from pathlib import Path

MAX_LINES = 90
plan = Path(__file__).resolve().parent.parent / "plan"
state = (plan / "STATE.md").read_text().splitlines()
rows_text = (plan / "rows.md").read_text()
problems = []

if len(state) > MAX_LINES:
    problems.append(f"STATE.md has {len(state)} lines (limit {MAX_LINES})")

# The rows of each table, with the heading of the table.
ids, status, ready_play, section = [], {}, 0, ""
for line in state:
    if line.startswith("## "):
        section = line[3:]
    m = re.match(r"\| (S-\d+) \|.*\| ([^|]*) \|$", line)
    if not m:
        continue
    row, stat = m.group(1), m.group(2)
    ids.append(row)
    status[row] = stat
    if section.startswith("Mikey plays") and "**ready**" in stat:
        ready_play += 1
    if section.startswith("Builder") and len(line.split("|")) != 7:
        problems.append(f"{row}: a Builder row needs the Play column")

for row in sorted(set(ids)):
    if ids.count(row) > 1:
        problems.append(f"{row} is in STATE.md {ids.count(row)} times")

detail = re.findall(r"^## (S-\d+)$", rows_text, re.M)
for row in ids:
    if row not in detail:
        problems.append(f"{row} is in STATE.md but not in rows.md")
for row in detail:
    if row not in ids:
        problems.append(f"{row} is in rows.md but not in STATE.md")
    if detail.count(row) > 1:
        problems.append(f"{row} is in rows.md {detail.count(row)} times")

# A row must not wait on a row that is gone.
for row, stat in status.items():
    if "waiting" in stat or "after" in stat:
        for other in re.findall(r"S-\d+", stat):
            if other not in ids:
                problems.append(f"{row} waits on {other}, which is not a row")

text = "\n".join(state)
m = re.search(r"Next number: S-(\d+)", text)
highest = max((int(r[2:]) for r in ids), default=0)
if not m:
    problems.append('STATE.md has no "Next number: S-NNN" line')
elif int(m.group(1)) <= highest:
    problems.append(f"Next number S-{m.group(1)} is not above S-{highest}")

# The play gate (R28): closed when two or more play rows are ready.
m = re.search(r"\*\*Play gate: (open|closed)\*\*", text)
want = "closed" if ready_play >= 2 else "open"
if not m:
    problems.append('STATE.md has no "**Play gate: open**" or "closed" line')
elif m.group(1) != want:
    problems.append(
        f"Play gate says {m.group(1)}, but {ready_play} play rows are ready ({want})"
    )

for p in sorted(set(problems)):
    print(f"plan: {p}")
if problems:
    sys.exit(1)
print(f"Plan check passed: {len(ids)} rows, play gate {want}.")
