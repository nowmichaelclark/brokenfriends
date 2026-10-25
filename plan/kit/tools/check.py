#!/usr/bin/env python3
"""Check the planning repo against kit/RULES.md.

    python3 kit/tools/check.py

Errors (exit code 1): broken links, missing or wrong statuses, locked
docs that still hold [unverified] claims, malformed decisions.
Warnings: files over their size budget, references to V-numbers that
verify.md doesn't list, and similar housekeeping.
"""

import re
import sys
from collections import Counter

from common import (
    DECISION_STATES, DOC_LADDER, FOLDER_LIMITS, GAME_README_LINES, HOT_SET,
    HOT_SET_LINES, SCRIPT_EXEMPT, SCRIPT_GLOBS, SCRIPT_LINES, LOCKED_STATES, PROTOTYPE_STATES, ROOT,
    UNVERIFIED_RE, V_REF_RE, budget_for, markdown_files, path_matches,
    read_frontmatter, rel, requires_doc_status, strip_code,
)

LINK_RE = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")
DECISION_NAME_RE = re.compile(r"^(\d{4})-[a-z0-9-]+\.md$")
VERIFY_LINE_RE = re.compile(r"^\s*- \[( |x|X)\] V-(\d{3,})\b")


def main():
    errors, warnings = [], []
    doc_status = Counter()
    decision_status = Counter()
    unverified_total = 0
    unverified_refs = {}  # V-number -> first file mentioning it
    decision_numbers = {}

    for path in markdown_files():
        r = rel(path)
        text = path.read_text(encoding="utf-8")
        lines = text.count("\n") + (0 if text.endswith("\n") else 1)
        templates = r.startswith("kit/templates/")

        # Size budgets
        limit = budget_for(r)
        if limit and lines > limit:
            warnings.append(f"{r}: {lines} lines, over its budget of {limit}. Split or cut it.")

        fm = read_frontmatter(text)
        body = strip_code(text)

        # Links
        if not templates:
            for target in LINK_RE.findall(body):
                if re.match(r"^[a-z]+:", target) or target.startswith("#"):
                    continue
                dest = (path.parent / target.split("#")[0]).resolve()
                if not dest.exists():
                    errors.append(f"{r}: broken link to '{target}'.")

        # [unverified] tags
        tags = UNVERIFIED_RE.findall(body)
        counted = (requires_doc_status(r) or path_matches(r, "decisions/*.md")) and not templates
        if counted:
            unverified_total += len(tags)
            for tag in tags:
                for num in V_REF_RE.findall(tag):
                    unverified_refs.setdefault(num, r)

        # Status-ladder docs
        if requires_doc_status(r) and not templates:
            status = (fm or {}).get("status", "")
            if fm is None:
                errors.append(f"{r}: no frontmatter. Start it with a '---' block holding 'status:'.")
            elif status not in DOC_LADDER:
                errors.append(f"{r}: status '{status}' is not one of {', '.join(DOC_LADDER)}.")
            else:
                doc_status[status] += 1
                if status in LOCKED_STATES and tags:
                    errors.append(f"{r}: status '{status}' but {len(tags)} [unverified] claim(s) remain.")

        # Decisions
        if path_matches(r, "decisions/*.md") and path.name != "README.md":
            m = DECISION_NAME_RE.match(path.name)
            if not m:
                errors.append(f"{r}: decision files are named NNNN-short-name.md.")
            else:
                num = m.group(1)
                if num in decision_numbers:
                    errors.append(f"{r}: number {num} is also used by {decision_numbers[num]}.")
                decision_numbers[num] = r
            status = (fm or {}).get("status", "")
            if status not in DECISION_STATES:
                errors.append(f"{r}: decision status '{status}' is not one of {', '.join(DECISION_STATES)}.")
            else:
                decision_status[status] += 1
                if status in {"accepted", "rejected"} and not fm.get("decided_by"):
                    errors.append(f"{r}: {status} but 'decided_by' is empty. Sign it 'Mikey, YYYY-MM-DD'.")
                if status == "superseded" and not fm.get("superseded_by"):
                    errors.append(f"{r}: superseded but 'superseded_by' is empty.")
                if status == "accepted" and tags:
                    warnings.append(f"{r}: accepted, with {len(tags)} [unverified] claim(s) still to check.")

        # Prototypes
        if path_matches(r, "prototypes/*.md") and path.name != "README.md":
            status = (fm or {}).get("status", "")
            if status not in PROTOTYPE_STATES:
                errors.append(f"{r}: prototype status '{status}' is not one of {', '.join(PROTOTYPE_STATES)}.")

    # Every system folder has a README
    systems = ROOT / "systems"
    if systems.is_dir():
        for folder in sorted(p for p in systems.iterdir() if p.is_dir()):
            if not (folder / "README.md").exists():
                errors.append(f"systems/{folder.name}/: missing README.md.")

    # verify.md
    open_v, done_v, listed = 0, 0, set()
    verify = ROOT / "verify.md"
    if verify.exists():
        for line in verify.read_text(encoding="utf-8").splitlines():
            m = VERIFY_LINE_RE.match(line)
            if m:
                if m.group(2) in listed:
                    errors.append(f"verify.md: V-{m.group(2)} is listed twice.")
                listed.add(m.group(2))
                if m.group(1) == " ":
                    open_v += 1
                else:
                    done_v += 1
    else:
        errors.append("verify.md is missing.")
    for num, where in sorted(unverified_refs.items()):
        if num not in listed:
            warnings.append(f"{where}: refers to V-{num}, which verify.md doesn't list.")

    if not (ROOT / "MIKEY.md").exists():
        errors.append("MIKEY.md is missing.")

    lean_checks(warnings)

    # Report
    for title, items in (("Errors", errors), ("Warnings", warnings)):
        if items:
            print(f"{title} ({len(items)}):")
            for item in items:
                print(f"  - {item}")
            print()

    print("Summary:")
    print("  Docs by status:      " + ", ".join(f"{s} {doc_status[s]}" for s in DOC_LADDER))
    print("  Decisions:           " + ", ".join(f"{s} {decision_status[s]}" for s in DECISION_STATES))
    print(f"  [unverified] tags:   {unverified_total}")
    print(f"  verify.md:           {open_v} open, {done_v} done")
    print()
    if errors:
        print(f"FAILED: {len(errors)} error(s).")
        return 1
    print("OK" + (f", with {len(warnings)} warning(s)." if warnings else "."))
    return 0


def _lines(path):
    text = path.read_text(encoding="utf-8")
    return text.count("\n") + (0 if text.endswith("\n") else 1)


def lean_checks(warnings):
    """Warnings that keep the repo from piling up (RULES.md section 6)."""
    for folder, most, fix in FOLDER_LIMITS:
        d = ROOT / folder
        if d.is_dir():
            files = [p for p in d.iterdir() if p.is_file() and p.name != "README.md"]
            if len(files) > most:
                warnings.append(f"{folder}/: {len(files)} files, more than {most}. Cleanup: {fix}.")
    hot = sum(_lines(ROOT / f) for f in HOT_SET if (ROOT / f).exists())
    if hot > HOT_SET_LINES:
        warnings.append(f"The files every session reads first ({', '.join(HOT_SET)}) total "
                        f"{hot} lines, over {HOT_SET_LINES}. Cut them.")
    repo = ROOT.parent
    readme = repo / "game" / "README.md"
    if readme.exists() and _lines(readme) > GAME_README_LINES:
        warnings.append(f"game/README.md: {_lines(readme)} lines, over {GAME_README_LINES}. "
                        "It says what the build is and assumes now, not what each step added.")
    for pattern in SCRIPT_GLOBS:
        for p in sorted(repo.glob(pattern)):
            if "target" in p.parts or p.relative_to(repo).as_posix() in SCRIPT_EXEMPT:
                continue
            n = _lines(p)
            if n > SCRIPT_LINES:
                warnings.append(f"{p.relative_to(repo).as_posix()}: {n} lines, over {SCRIPT_LINES}. "
                                "Split it before adding more.")


if __name__ == "__main__":
    sys.exit(main())
