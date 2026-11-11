#!/usr/bin/env python3
"""Build the labeling fixtures for the further-evaluation seams.

Each fixture writes a markdown file with a numbered, copy-and-judge
table: the real row, the candidate answer key, and a blank column. The
owner fills the blank; scripts/score-fixtures.py grades the head
against it.

  fixtures/c13-mention.md   30 owner messages -> ask | directive | review
  fixtures/c13-silent.md    20 tool-using turns that ended bare -> reported | silent
  fixtures/c14-skills.md    10 candidate skill pairs -> merge | keep
  fixtures/c14-lessons.md   3 lesson/practice pairs -> covers | misses
  fixtures/c15-recall.md    50 instruction->recalled-fact pairs -> want | skip
  fixtures/c16-queues.md    30 queue rows -> high | low / worth | stale / ready | not
"""

from __future__ import annotations

import json
import random
import re
from pathlib import Path

# The owner's home, never the repo: the fixtures carry real session
# text and the repo is public.
OUT = Path.home() / ".fleet" / "fixtures"
SESSIONS = Path.home() / ".fleet" / "sessions"


def logs():
    for path in sorted(SESSIONS.glob("*.jsonl")):
        try:
            yield path, path.read_text(errors="replace").splitlines()
        except OSError:
            continue


def records(path, lines):
    for line in lines:
        if not line.strip():
            continue
        try:
            yield json.loads(line)
        except json.JSONDecodeError:
            continue


def c13_mention(limit=30):
    rows = []
    rng = random.Random(11)
    for path, lines in logs():
        for rec in records(path, lines):
            if rec.get("type") != "user":
                continue
            text = (rec.get("content") or "").strip()
            if 30 <= len(text) <= 400 and "\n" not in text:
                rows.append((path.stem, text))
    rng.shuffle(rows)
    rows = rows[:limit]
    out = ["# C13 — mention kind\n",
           "Label each owner message: **ask** (wanted information), **directive** ("
           "ordered work done), or **review** (judged work already done).\n",
           "| # | session | message | your label |", "|---|---|---|---|"]
    for i, (sid, text) in enumerate(rows, 1):
        out.append(f"| {i} | {sid[:17]} | {text[:160].replace('|', '/')} | |")
    (OUT / "c13-mention.md").write_text("\n".join(out) + "\n")
    return len(rows)


def c13_silent(limit=20):
    rows = []
    for path, lines in logs():
        used_tool = ended_bare = False
        last_text = ""
        for rec in records(path, lines):
            kind = rec.get("type")
            if kind == "assistant":
                calls = rec.get("tool_calls") or []
                if calls:
                    used_tool = True
                content = (rec.get("content") or "").strip()
                if content:
                    last_text = content
            elif kind == "end":
                ended_bare = used_tool and len(last_text) < 200
        if ended_bare:
            rows.append((path.stem, last_text[:160]))
    rows = rows[:limit]
    out = ["# C13 — silent stop\n",
           "For each turn that used tools and ended on a short/empty text: did the "
           "assistant actually **report** its result, or stop **silent**?\n",
           "| # | session | final text | your call |", "|---|---|---|---|"]
    for i, (sid, text) in enumerate(rows, 1):
        shown = text if text else "(no text)"
        out.append(f"| {i} | {sid[:17]} | {shown.replace('|', '/')} | |")
    (OUT / "c13-silent.md").write_text("\n".join(out) + "\n")
    return len(rows)


def c14_skills(limit=10):
    skills = []
    for d in sorted((Path.home() / ".agents" / "skills").glob("*/SKILL.md")):
        try:
            text = d.read_text(errors="replace")
        except OSError:
            continue
        m = re.search(r"^description:\s*(.+)$", text, re.M)
        if m:
            skills.append((d.parent.name, m.group(1).strip()[:200]))
    # candidate pairs: the closest by token overlap (a real curation shortlist)
    pairs = []
    for i, (a, da) in enumerate(skills):
        for b, db in skills[i + 1:]:
            ta, tb = set(da.lower().split()), set(db.lower().split())
            if not ta or not tb:
                continue
            j = len(ta & tb) / len(ta | tb)
            pairs.append((j, a, da, b, db))
    pairs.sort(reverse=True)
    pairs = pairs[:limit]
    out = ["# C14 — skill pairs\n",
           "Are these two skills the same capability? **merge** (one should absorb the "
           "other) or **keep** (genuinely different).\n",
           "| # | skill A | its description | skill B | its description | your call |",
           "|---|---|---|---|---|---|"]
    for i, (j, a, da, b, db) in enumerate(pairs, 1):
        out.append(f"| {i} | `{a}` | {da[:110].replace('|','/')} | `{b}` | {db[:110].replace('|','/')} | |")
    (OUT / "c14-skills.md").write_text("\n".join(out) + "\n")
    return len(pairs)


def c15_recall(limit=50):
    """Real recall calls where the model read a fact back — the pair is
    (instruction, fact name)."""
    rows = []
    rng = random.Random(15)
    for path, lines in logs():
        last_user = ""
        for rec in records(path, lines):
            kind = rec.get("type")
            if kind == "user":
                last_user = (rec.get("content") or "").strip()[:300]
            elif kind == "tool_result":
                out = rec.get("output") or ""
                if rec.get("name") == "recall" and not out.startswith(("error", "denied")):
                    # the card shape: "- <name> — <description> [score kind date]"
                    for match in re.finditer(r"^- ([a-z0-9][a-z0-9-]{5,60}) — (.{0,150})", out, re.M):
                        if last_user:
                            rows.append((last_user, match.group(1), match.group(2).strip()))
                    # the query the recall was asked for is the better instruction
                    for rec2 in []:
                        pass
    rng.shuffle(rows)
    rows = rows[:limit]
    out = ["# C15 — recall relevance\n",
           "Would you have wanted this fact surfaced for this instruction? "
           "**want** or **skip**.\n",
           "| # | instruction | fact | what it says | your call |", "|---|---|---|---|---|"]
    for i, (instr, fact, desc) in enumerate(rows, 1):
        out.append(f"| {i} | {instr[:110].replace('|','/')} | `{fact}` | {desc[:90].replace('|','/')} | |")
    (OUT / "c15-recall.md").write_text("\n".join(out) + "\n")
    return len(rows)


ANSI = re.compile(r"\x1b\[[0-9;]*m")
CARD = re.compile(r"^\s*\u250c\u2500\s*(.+?)\s*\u2500+\s*(\w+)\s*\u2510")


def cards(text: str, limit: int = 10):
    """The card shape the fleet verbs render: ┌─ name ─── status ┐."""
    found = []
    for line in text.splitlines():
        clean = ANSI.sub("", line)
        m = CARD.match(clean)
        if m:
            found.append((m.group(1).strip(), m.group(2).strip()))
    return found[:limit]


def c16_queues():
    """The three queue surfaces' real rows, one blank column each."""
    import subprocess
    blocks, total = [], 0

    def fleet(*args):
        try:
            return subprocess.run([str(Path.home() / ".fleet/bin/fleet"), *args],
                                  capture_output=True, text=True, timeout=60).stdout
        except Exception:
            return ""

    surfaces = [
        ("cron", "still worth firing when it catches up?", "worth / stale"),
        ("graph", "is this handoff ready?", "ready / not"),
        ("improve", "high or low priority?", "high / low"),
    ]
    for verb, question, key in surfaces:
        rows = cards(fleet(verb))
        blocks.append(f"## {verb} — {question}\n\nLabel each: **{key}**.\n\n"
                      f"| # | row | state | your call |\n|---|---|---|---|")
        for i, (name, state) in enumerate(rows, 1):
            blocks.append(f"| {i} | `{name}` | {state} | |")
            total += 1
    (OUT / "c16-queues.md").write_text("# C16 — queue triage\n\n" + "\n".join(blocks) + "\n")
    return total


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    counts = {
        "c13-mention": c13_mention(),
        "c13-silent": c13_silent(),
        "c14-skills": c14_skills(),
        "c15-recall": c15_recall(),
        "c16-queues": c16_queues(),
    }
    print("fixtures written to", OUT)
    for name, n in counts.items():
        print(f"  {name}.md — {n} row(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
