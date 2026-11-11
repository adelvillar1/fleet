#!/usr/bin/env python3
"""Grade a decision head against the owner's fixture labels.

Reads the labeled tables from ~/.fleet/fixtures, asks the vendor's head
the same question per row, and prints the head's agreement with the
owner beside the string rule's — the per-seam gate (C13-C16). A seam
builds only if the head beats the rule on the owner's own labels.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "services/sys1/src"))
from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_choice  # noqa: E402

FIXTURES = Path.home() / ".fleet" / "fixtures"

SEAMS = {
    "c13-mention.md": {
        "head": "Label each owner message: ask / directive / review.",
        "labels": ["ask", "directive", "review"],
        "rule": lambda text: (
            "review" if re.search(r"\b(review|critique|audit)\b", text[:80], re.I)
            else "directive" if re.search(r"^(run|fix|build|make|write|add|commit|push|do)\b", text, re.I)
            else "ask"
        ),
        "text_col": 2,
    },
    "c13-silent.md": {
        "head": "Did the turn report its result, or stop silent?",
        "labels": ["reported", "silent"],
        "rule": lambda text: "reported" if len(text.strip()) > 40 else "silent",
        "text_col": 2,
    },
    "c14-skills.md": {
        "head": "Are these two skills the same capability? merge / keep.",
        "labels": ["merge", "keep"],
        "rule": lambda text: "merge",  # the Jaccard shortlist already selected the pair
        "text_col": 1,
    },
    "c15-recall.md": {
        "head": "Would this fact be wanted for this instruction? want / skip.",
        "labels": ["want", "skip"],
        "rule": lambda text: "want",  # the channel ranking already surfaced it
        "text_col": 2,
    },
    "c16-queues.md": {
        "head": "Does this row deserve action now (high / low)?",
        # the three surfaces ask in their own words; high/low is the axis
        "labels": ["high", "low", "worth", "stale", "ready", "not"],
        "rule": lambda text: "high" if re.search(r"pending|enabled", text) else "low",
        "text_col": 1,
        "positive": {"high", "worth", "ready"},
    },
}


def rows(path: Path):
    """The labeled rows: numeric first cell + a non-empty last cell."""
    for line in path.read_text().splitlines():
        if not line.startswith("|") or set(line) <= set("|- "):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if not cells or not cells[0].isdigit():
            continue
        yield cells


def main() -> int:
    provider = sys.argv[1] if len(sys.argv) > 1 else "glide"
    labels_seen = 0
    for name, seam in SEAMS.items():
        path = FIXTURES / name
        if not path.exists():
            print(f"{name}: absent")
            continue
        graded = []
        for cells in rows(path):
            label = cells[-1].strip().lower()
            if label not in seam["labels"]:
                continue
            text = " ".join(c for c in cells[1:-1])
            graded.append((text, label, seam["rule"](text)))
        if not graded:
            print(f"{name}: 0 labeled row(s) — skip")
            continue
        labels_seen += len(graded)
        task = Task(
            id="fixture_seam",
            heads=[make_choice("fixture_seam", seam["labels"], id="answer")],
            description=seam["head"],
        )
        # Cross-vocabulary seams (C16's three surfaces) compare on the
        # positive/negative axis, not literal label strings.
        positive = seam.get("positive")

        def axis(value: str) -> str:
            if positive:
                return "positive" if value in positive else "negative"
            return value

        head_right = rule_right = 0
        for text, label, rule in graded:
            answer = classify(provider=provider, task=task, text=text[:3000]).head("answer", provider)
            head = (answer or {}).get("label", "?")
            head_right += int(axis(head) == axis(label))
            rule_right += int(axis(rule) == axis(label))
        n = len(graded)
        verdict = "BUILD (head beats the rule)" if head_right > rule_right else "KILL (the rule holds)"
        print(f"{name}: {n} labeled · head {head_right}/{n} ({head_right/n*100:.0f}%) · "
              f"rule {rule_right}/{n} ({rule_right/n*100:.0f}%) → {verdict}")
    if labels_seen == 0:
        print("\nNo labels yet — fill the 'your label/call' column in ~/.fleet/fixtures/ then re-run.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
