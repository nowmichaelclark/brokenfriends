#!/usr/bin/env python3
"""Write the files a model returned into the repo, safely.

    python3 kit/tools/unpack.py reply.txt
    wl-paste | python3 kit/tools/unpack.py -
    python3 kit/tools/unpack.py reply.txt --dry-run

Looks for blocks like:

    === FILE: systems/condition/README.md ===
    (whole file)
    === END FILE ===

and writes each one. It refuses, file by file, anything that breaks the
rules: MIKEY.md; locked docs; kit/ and tool files (unless --allow-kit);
paths outside the repo; files that look truncated ("rest unchanged");
a model setting a doc to locked or a decision to accepted (Mikey does
that himself, or reruns with --force). Nothing is committed: check the
result with `git diff` and `python3 kit/tools/check.py`.
"""

import argparse
import re
import sys
from pathlib import Path, PurePosixPath

from common import LOCKED_STATES, ROOT, path_matches, read_frontmatter

START_RE = re.compile(r"^ {0,3}=== FILE:\s*(.+?)\s*===\s*$")
END_RE = re.compile(r"^ {0,3}=== END FILE ===\s*$")
FENCE_RE = re.compile(r"^ {0,3}(```|~~~)")
HEADING_RE = re.compile(r"^#{1,6} \S", re.M)
RENDERED_BULLET_RE = re.compile(r"^\s*[·•●▪◦] ", re.M)
TRUNCATION_RE = re.compile(
    r"(rest of (the )?(file|doc|document|section)|remaining sections?|"
    r"(stays|remains|is|are) (the same|unchanged)|\(unchanged\)|\[unchanged\]|"
    r"as before\)|no changes below)",
    re.IGNORECASE,
)
ALWAYS_REFUSED = {"MIKEY.md"}
KIT_PREFIXES = ("kit/", ".github/", ".claude/")


def parse(text):
    """Return (blocks, notes, problems). blocks: list of (path, content)."""
    blocks, notes, problems = [], [], []
    current, buf = None, []
    for line in text.splitlines():
        start, end = START_RE.match(line), END_RE.match(line)
        if start:
            if current is not None:
                problems.append(f"{current}: no END FILE line before the next file; skipped.")
            current, buf = start.group(1).strip("`'\" "), []
        elif end and current is not None:
            blocks.append((current, clean("\n".join(buf))))
            current, buf = None, []
        elif current is not None:
            buf.append(line)
        else:
            notes.append(line)
    if current is not None:
        problems.append(f"{current}: the reply ends before END FILE (cut off?); skipped.")
    return blocks, notes, problems


def clean(content):
    """Drop one outer code fence if the model wrapped the file in one."""
    lines = content.split("\n")
    nonblank = [i for i, l in enumerate(lines) if l.strip()]
    if len(nonblank) >= 2:
        first, last = nonblank[0], nonblank[-1]
        opening = FENCE_RE.match(lines[first])
        if opening and lines[last].strip().startswith(opening.group(1)) \
                and set(lines[last].strip()) <= set("`~"):
            lines = lines[first + 1:last]
    return "\n".join(lines).strip("\n") + "\n"


def check_path(raw, args):
    p = PurePosixPath(raw)
    if p.parts[:1] == ("plan",) and len(p.parts) > 1:
        p = PurePosixPath(*p.parts[1:])  # paths are relative to plan/
    if p.is_absolute() or ".." in p.parts or not p.parts:
        return None, "path is outside the repo"
    r = p.as_posix()
    if p.parts[0] in {".git", "packs"}:
        return None, "not a planning file"
    if r in ALWAYS_REFUSED:
        return None, "MIKEY.md is Mikey's alone"
    if r.startswith(KIT_PREFIXES) and not args.allow_kit:
        return None, "kit and tool files change only with --allow-kit"
    if p.suffix != ".md" and not args.force:
        return None, "only .md files (use --force to override)"
    return r, None


def check_content(r, new, args):
    """Return a refusal reason or None, plus a list of warnings."""
    warnings = []
    target = ROOT / r
    old = target.read_text(encoding="utf-8") if target.exists() else None
    old_fm = read_frontmatter(old) if old else None
    new_fm = read_frontmatter(new) or {}
    old_status = (old_fm or {}).get("status", "")
    new_status = new_fm.get("status", "")
    is_decision = path_matches(r, "decisions/*.md")

    placeholder = [l.strip() for l in new.splitlines()
                   if TRUNCATION_RE.search(l) and l.strip()[:1] in "[(<…*_.{"]
    loose = [l.strip() for l in new.splitlines()
             if TRUNCATION_RE.search(l) and l.strip() not in placeholder]
    for line in loose:
        warnings.append(f"possible placeholder line: '{line[:70]}'")

    rendered = None
    if r.endswith(".md"):
        if not HEADING_RE.search(new):
            rendered = "has no markdown heading (#)"
        elif RENDERED_BULLET_RE.search(new):
            rendered = "has bullets like '·' instead of '-'"

    if not args.force:
        if rendered:
            return (f"{rendered}: it was probably copied after the chat rendered it. "
                    "Ask the model to put all files in one ~~~~ code block, "
                    "and copy with that block's copy button"), warnings
        if placeholder:
            return (f"looks like a partial file ('{placeholder[0][:60]}'); "
                    "ask for the whole file"), warnings
        if old_status in LOCKED_STATES and not is_decision:
            return f"the doc is {old_status}; only Mikey changes it (rerun with --force if he has)", warnings
        if new_status in LOCKED_STATES and old_status not in LOCKED_STATES and not is_decision:
            return "a model can't lock a doc; set the status yourself", warnings
        if is_decision:
            if old_status in {"accepted", "rejected"} and new_status != "superseded":
                return f"decision is {old_status}; it can only be marked superseded", warnings
            if new_status in {"accepted", "rejected"} and old_status != new_status:
                return "only Mikey accepts or rejects a decision (rerun with --force if he did)", warnings

    if old is not None:
        old_n, new_n = old.count("\n"), new.count("\n")
        if old_n >= 20 and new_n < old_n * 0.5:
            warnings.append(f"shrank from {old_n} to {new_n} lines; check nothing was dropped")
    return None, warnings


def main():
    ap = argparse.ArgumentParser(description="Unpack files from a model's reply.")
    ap.add_argument("reply", help="file with the model's reply, or - for stdin")
    ap.add_argument("--dry-run", action="store_true", help="show what would happen, write nothing")
    ap.add_argument("--force", action="store_true", help="skip the safety refusals (not MIKEY.md)")
    ap.add_argument("--allow-kit", action="store_true", help="allow writing kit/, .github/, .claude/")
    args = ap.parse_args()

    text = sys.stdin.read() if args.reply == "-" else Path(args.reply).read_text(encoding="utf-8")
    blocks, notes, problems = parse(text)
    if not blocks and not problems:
        sys.exit("unpack: no '=== FILE: path ===' blocks found in the reply.")

    written, unchanged, refused = [], [], list(problems)
    for raw, content in blocks:
        r, reason = check_path(raw, args)
        if reason:
            refused.append(f"{raw}: {reason}.")
            continue
        reason, warnings = check_content(r, content, args)
        if reason:
            refused.append(f"{r}: {reason}.")
            continue
        target = ROOT / r
        if target.exists() and target.read_text(encoding="utf-8") == content:
            unchanged.append(r)
            continue
        kind = "changed" if target.exists() else "new"
        if not args.dry_run:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(content, encoding="utf-8")
        written.append((r, kind, warnings))

    verb = "Would write" if args.dry_run else "Wrote"
    if written:
        print(f"{verb} {len(written)} file(s):")
        for r, kind, warnings in written:
            print(f"  + {r} ({kind})")
            for w in warnings:
                print(f"      warning: {w}")
    if unchanged:
        print(f"Unchanged: {', '.join(unchanged)}")
    if refused:
        print(f"Refused {len(refused)}:")
        for item in refused:
            print(f"  ! {item}")

    notes = [n for n in notes if not FENCE_RE.match(n)]
    note_text = re.sub(r"\n\s*\n+", "\n\n", "\n".join(notes)).strip()
    if "weak spots" not in text.lower():
        print("\nNote: the reply has no 'Weak spots' section. Ask the model for one.")
    if note_text:
        lines = note_text.splitlines()
        print("\nThe model's notes (not saved):")
        for line in lines[:25]:
            print(f"  | {line}")
        if len(lines) > 25:
            print(f"  | ... ({len(lines) - 25} more lines)")

    if written and not args.dry_run:
        print("\nNext: git diff, then python3 kit/tools/check.py, then commit.")
    return 1 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
