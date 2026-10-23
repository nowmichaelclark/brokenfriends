"""Shared helpers for the planning tools. Standard library only."""

import re
from fnmatch import fnmatchcase
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]

DOC_LADDER = ["idea", "draft", "review", "locked", "prototyped"]
LOCKED_STATES = {"locked", "prototyped"}
DECISION_STATES = ["proposed", "accepted", "rejected", "superseded"]
PROTOTYPE_STATES = ["planned", "building", "played", "closed"]

# Line budgets from kit/RULES.md, section 6. First match wins.
BUDGETS = [
    ("START-HERE.md", 120),
    ("STATE.md", 100),
    ("00-vision/*.md", 150),
    ("systems/*/*.md", 300),
    ("systems-map.md", 250),
    ("glossary.md", 300),
    ("decisions/*.md", 80),
    ("open-questions.md", 200),
    ("verify.md", 200),
    ("reviews/*.md", 200),
    ("research/*.md", 200),
    ("prototypes/*.md", 150),
    ("log/*.md", 40),
    ("inbox/*.md", 200),
]

UNVERIFIED_RE = re.compile(r"\[unverified[^\]]*\]", re.IGNORECASE)
V_REF_RE = re.compile(r"\bV-(\d{3,})\b")


def rel(path):
    """Repo-relative POSIX path as a string."""
    return Path(path).resolve().relative_to(ROOT).as_posix()


def path_matches(relpath, pattern):
    """Glob match where '*' never crosses a '/'."""
    a = PurePosixPath(relpath).parts
    b = PurePosixPath(pattern).parts
    return len(a) == len(b) and all(fnmatchcase(x, y) for x, y in zip(a, b))


def budget_for(relpath):
    for pattern, limit in BUDGETS:
        if path_matches(relpath, pattern):
            return limit
    return None


def read_frontmatter(text):
    """Parse a simple 'key: value' frontmatter block. Returns a dict, or
    None when the text has no frontmatter."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return None
    data = {}
    for line in lines[1:]:
        if line.strip() == "---":
            return data
        if ":" in line:
            key, _, value = line.partition(":")
            data[key.strip()] = value.strip()
    return None  # never closed


def strip_code(text):
    """Remove fenced code blocks and inline code, so instructions that
    quote a tag or a link aren't counted as real ones."""
    text = re.sub(r"^```.*?^```[^\n]*$", "", text, flags=re.S | re.M)
    return re.sub(r"`[^`\n]*`", "", text)


def requires_doc_status(relpath):
    """Docs that climb the status ladder."""
    if relpath == "systems-map.md":
        return True
    if path_matches(relpath, "00-vision/*.md"):
        return True
    return path_matches(relpath, "systems/*/*.md")


def markdown_files():
    for p in sorted(ROOT.rglob("*.md")):
        parts = p.relative_to(ROOT).parts
        if parts[0] in {".git", "packs", "node_modules"}:
            continue
        yield p
