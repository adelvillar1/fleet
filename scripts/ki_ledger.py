#!/usr/bin/env python3
"""Parse docs/known-issues.md and report every row missing a required field.

The ledger is a markdown table whose rows carry six pipe-separated cells:
ID | severity | description | evidence | status | next action. A row is
flagged when any of ID, severity, status, or next action is empty, or when
the row's cell count disagrees with the header's — the signature of an
unescaped pipe inside a code span shifting the cells (KI-084).
"""
import re
import sys
from pathlib import Path

HEADER = "| ID | Severity | Issue | Evidence | Status | Next action |"


def parse(path: Path):
    text = path.read_text(encoding="utf-8")
    rows, header_cells, in_rules = [], None, False
    for lineno, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if not stripped:
            in_rules = False
            continue
        if stripped.startswith("## Rules for this ledger"):
            in_rules = True
            continue
        if in_rules:
            continue
        if not stripped.startswith("|"):
            continue
        cells = split_cells(stripped)
        if stripped.startswith("| ID |"):
            header_cells = cells
            continue
        if not stripped.startswith("| KI-"):
            continue
        if stripped.startswith("| ID |"):
            header_cells = cells
            continue
        if all(re.fullmatch(r":?-{2,}:?", c.strip()) for c in cells if c.strip()):
            continue
        rows.append((lineno, stripped, cells))
    return header_cells, rows


def split_cells(line: str):
    """Split a table row on the pipes that are not backslash-escaped.

    The leading and trailing pipes delimit the row and are not separators:
    `| a | b |` is two cells, not three.
    """
    body = line.strip()
    if body.startswith("|"):
        body = body[1:]
    if body.endswith("|") and not body.endswith("\\|"):
        body = body[:-1]
    out, buf = [], ""
    i = 0
    while i < len(body):
        ch = body[i]
        if ch == "\\" and i + 1 < len(body) and body[i + 1] == "|":
            buf += "|"
            i += 2
            continue
        if ch == "|":
            out.append(buf.strip())
            buf = ""
            i += 1
            continue
        buf += ch
        i += 1
    out.append(buf.strip())
    return out


def is_open(status: str) -> bool:
    """A row is open when its status cell says open, markdown emphasis aside.

    The ledger writes statuses as `open`, `**open — reason**`, `open (deferred)`,
    and the closed variants, so the leading emphasis and any trailing gloss are
    stripped before the word is read.
    """
    word = status.strip().strip("*_`").strip().lower()
    return word.startswith("open")


def main() -> int:
    path = Path(sys.argv[1] if len(sys.argv) > 1 else "docs/known-issues.md")
    if not path.exists():
        print(f"FAIL: {path} not found")
        return 2
    header, rows = parse(path)
    problems = []
    seen = {}
    for lineno, raw, cells in rows:
        if len(cells) != len(header):
            problems.append(
                f"{path}:{lineno}: cell count {len(cells)} != header {len(header)} "
                f"— an unescaped pipe is shifting this row"
            )
            padded = cells + [""] * (len(header) - len(cells))
        else:
            padded = cells
        ki_id, severity, status, nxt = padded[0], padded[1], padded[4], padded[5]
        if not re.fullmatch(r"KI-\d{3}", ki_id):
            problems.append(f"{path}:{lineno}: bad or missing ID {ki_id!r}")
        if not severity:
            problems.append(f"{path}:{lineno} ({ki_id}): missing severity")
        if not status:
            problems.append(f"{path}:{lineno} ({ki_id}): missing status")
        if not nxt:
            problems.append(f"{path}:{lineno} ({ki_id}): missing next action")
        if ki_id in seen:
            problems.append(
                f"{path}:{lineno} ({ki_id}): duplicate ID, first seen at line {seen[ki_id]}"
            )
        seen[ki_id] = lineno

    open_rows = [r for r in rows if len(r[2]) >= 5 and is_open(r[2][4])]
    print(f"rows: {len(rows)}   open: {len(open_rows)}")
    if problems:
        print(f"PROBLEMS ({len(problems)}):")
        for p in problems:
            print("  " + p)
        return 1
    print("zero rows missing ID/severity/status/next-action; all cell counts agree with the header")
    return 0


if __name__ == "__main__":
    sys.exit(main())
