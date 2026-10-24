#!/usr/bin/env python3
"""Build a session pack: one block of text to paste into any chat model.

    python3 kit/tools/pack.py SESSION [FILES_OR_FOLDERS ...] --task "..."

Every pack holds START-HERE.md, kit/RULES.md, STATE.md, glossary.md,
open-questions.md, verify.md, the decision and session-log templates and
the session's brief, then the files you name (folders expand to their
.md files), then your task, at the top and again at the end.

Examples:
    python3 kit/tools/pack.py brain 00-vision inbox --task "Let's work out the pitch."
    python3 kit/tools/pack.py red-team 00-vision --with-decisions --copy
    python3 kit/tools/pack.py drafter systems/condition kit/templates/system.md --task-file task.txt

The pack is saved in packs/ (ignored by git). --copy also puts it on the
clipboard; --stdout prints it.
"""

import argparse
import datetime
import shutil
import subprocess
import sys
from pathlib import Path

from common import ROOT, read_frontmatter, rel

ALWAYS = ["START-HERE.md", "kit/RULES.md", "STATE.md", "glossary.md",
          "open-questions.md", "verify.md",
          "kit/templates/decision.md", "kit/templates/session-log.md"]
SESSIONS = sorted(p.stem for p in (ROOT / "kit" / "sessions").glob("*.md"))
NO_TASK = "(No task given. Mikey, replace this line with today's task before pasting.)"


def resolve(arg):
    """Turn an argument into a list of files inside the repo."""
    for base in (Path.cwd(), ROOT):
        p = (base / arg).resolve()
        if p.exists():
            break
    else:
        matches = sorted(ROOT.glob(arg))
        if not matches:
            sys.exit(f"pack: nothing found for '{arg}'.")
        return [m for m in matches if m.is_file()]
    if ROOT not in p.parents and p != ROOT:
        sys.exit(f"pack: '{arg}' is outside the repo.")
    if p.is_dir():
        return sorted(f for f in p.rglob("*.md") if ".git" not in f.parts)
    return [p]


def decisions(include_all):
    out = []
    for p in sorted((ROOT / "decisions").glob("[0-9]*.md")):
        status = (read_frontmatter(p.read_text(encoding="utf-8")) or {}).get("status")
        if include_all or status == "accepted":
            out.append(p)
    return out


def copy_to_clipboard(text):
    for cmd in (["wl-copy"], ["xclip", "-selection", "clipboard"],
                ["xsel", "--clipboard", "--input"], ["pbcopy"]):
        if shutil.which(cmd[0]):
            subprocess.run(cmd, input=text.encode("utf-8"), check=True)
            return cmd[0]
    return None


def main():
    ap = argparse.ArgumentParser(description="Build a paste-able session pack.")
    ap.add_argument("session", choices=SESSIONS, help="which brief in kit/sessions/")
    ap.add_argument("paths", nargs="*", help="files, folders or globs to include")
    ap.add_argument("--task", help="today's task, in your words")
    ap.add_argument("--task-file", help="read the task from a file")
    ap.add_argument("--with-decisions", action="store_true", help="include accepted decisions")
    ap.add_argument("--all-decisions", action="store_true", help="include every decision")
    ap.add_argument("--out", help="where to save the pack (default: packs/)")
    ap.add_argument("--copy", action="store_true", help="also copy it to the clipboard")
    ap.add_argument("--stdout", action="store_true", help="print the pack")
    ap.add_argument("--warn-tokens", type=int, default=60000,
                    help="warn above this rough token count (default 60000)")
    args = ap.parse_args()

    task = args.task
    if args.task_file:
        task = Path(args.task_file).read_text(encoding="utf-8").strip()
    task = task or NO_TASK

    files = [ROOT / f for f in ALWAYS] + [ROOT / "kit" / "sessions" / f"{args.session}.md"]
    for arg in args.paths:
        files += resolve(arg)
    if args.with_decisions or args.all_decisions:
        files += decisions(args.all_decisions)
    seen, ordered = set(), []
    for f in files:
        if f.exists() and f not in seen:
            seen.add(f)
            ordered.append(f)

    today = datetime.date.today().isoformat()
    blocks, listing, total_lines = [], [], 0
    for f in ordered:
        content = f.read_text(encoding="utf-8").rstrip("\n")
        n = content.count("\n") + 1
        total_lines += n
        listing.append(f"- {rel(f)} ({n} lines)")
        blocks.append(f"=== FILE: {rel(f)} ===\n{content}\n=== END FILE ===")

    pack = "\n".join([
        f"# Session pack: {args.session} ({today})",
        "",
        f"You are joining a game-planning project as a **{args.session}** session.",
        "Read every file below before you answer. START-HERE.md and kit/RULES.md",
        f"are the rules; kit/sessions/{args.session}.md is your brief.",
        'Each file sits between a "=== FILE: path ===" line and an "=== END FILE ===" line.',
        "When you return files, use exactly that format, whole files only.",
        "",
        "## Task from Mikey",
        "",
        task,
        "",
        "## Files in this pack",
        "",
        *listing,
        "",
        "\n\n".join(blocks),
        "",
        "## The task again",
        "",
        task,
        "",
        "Follow your brief. Talk first; return files only when Mikey asks or the",
        "task is finished. Then put every file inside ONE code block fenced with",
        "~~~~ lines (see kit/RULES.md, section 2). End your reply with **Weak spots**.",
        "",
    ])

    out = Path(args.out) if args.out else ROOT / "packs" / f"{today}-{args.session}.md"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(pack, encoding="utf-8")

    tokens = len(pack) // 4
    print(f"Pack saved: {out}", file=sys.stderr)
    print(f"  {len(ordered)} files, {total_lines} lines, {len(pack):,} characters, "
          f"roughly {tokens:,} tokens", file=sys.stderr)
    if tokens > args.warn_tokens:
        print("  Warning: that's large for many free models. Consider splitting the task.",
              file=sys.stderr)
    if task == NO_TASK:
        print("  Warning: no --task given; edit the pack's task lines before pasting.",
              file=sys.stderr)
    if args.copy:
        tool = copy_to_clipboard(pack)
        print(f"  Copied with {tool}." if tool else
              "  No clipboard tool found (install wl-clipboard). Open the file instead.",
              file=sys.stderr)
    if args.stdout:
        print(pack)


if __name__ == "__main__":
    main()
