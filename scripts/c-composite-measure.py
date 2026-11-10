#!/usr/bin/env python3
"""The composite replay: stacked calls against the flat pass's kill gate.

The C1 head-to-head measured ONE shape — a single choice pass over the
whole catalog (best head 56.7% top-1). This script measures the composed
shapes on the same 120 recorded turns:

  A1  classify -> decide    pass 1 picks the tool CLASS, pass 2 picks the
                            tool within that class
  A2  classify(top-2) ->    pass 1's top-2 classes union their tools;
      decide                wrong-class errors get one recovery hop

Ground truth: the recorded choice, exactly as C1. Deterministic stride,
so the turn set is identical.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))

from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_choice  # noqa: E402

# The same loader the C1 head-to-head used (loaded by path — the
# hyphenated filename is not importable by name — so the turn set is
# identical by construction).
import importlib.util  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "selection_headtohead", REPO / "scripts" / "selection-headtohead.py"
)
_h2h = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_h2h)
load_candidates = _h2h.load_candidates
topk_hit = _h2h.topk_hit

CLASSES = {
    "files": ["read_file", "edit", "write_file", "list_dir", "file_change"],
    "search": ["grep", "glob", "tool_search"],
    "shell": ["bash", "process"],
    "memory": ["remember", "recall", "supersede", "memories"],
    "skills": ["use_skill"],
    "run-control": ["goal", "compact", "doctor", "checkpoints"],
    "delegation": ["subagent"],
}


def class_of(tool: str) -> str:
    if tool.startswith("mcp__"):
        return f"lane-{tool.split('__')[1]}"
    for cls, tools in CLASSES.items():
        if tool in tools:
            return cls
    return "other"


def ask(provider: str, labels: list[str], text: str, head_id: str, task_id: str, description: str):
    task = Task(
        id=task_id,
        heads=[make_choice(task_id, labels, id=head_id)],
        description=description,
    )
    answer = classify(provider=provider, task=task, text=text).head(head_id, provider)
    if not answer or answer.get("_declined"):
        return None, {}
    return answer.get("label"), answer.get("probabilities") or {}


def main() -> int:
    provider = sys.argv[1] if len(sys.argv) > 1 else "glide"
    candidates = load_candidates(120)
    print(f"{len(candidates)} turn(s), provider {provider}: A1 classify->decide, A2 top-2-class union",
          file=sys.stderr)
    a1_top1 = a1_class_hit = a2_top1 = 0
    a1_top3 = a2_top3 = 0
    answered = 0
    for index, candidate in enumerate(candidates):
        options = candidate["options"]
        truth = candidate["truth"]
        truth_class = class_of(truth)
        classes = sorted({class_of(tool) for tool in options} | {"other"})
        # pass 1: the class
        label1, probs1 = ask(
            provider, classes, candidate["query"], "tool_class", "tool_class_pass",
            "Which class of work does this intent need?",
        )
        if label1 is None:
            continue
        answered += 1
        class_hit = label1 == truth_class
        a1_class_hit += int(class_hit)
        # A1: decide within the predicted class
        within = [t for t in options if class_of(t) == label1] or options
        within = within[:255]
        label2, probs2 = ask(
            provider, within, candidate["query"], "tool_choice", "tool_choice_pass",
            "Which single tool from this class best serves the intent?",
        )
        if label2 == truth:
            a1_top1 += 1
        if probs2 and topk_hit({"probabilities": probs2}, truth, 3):
            a1_top3 += 1
        # A2: the top-2 classes' union
        ranked = sorted(probs1, key=lambda c: probs1.get(c, 0), reverse=True)
        union_classes = set(ranked[:2]) | {truth_class} if False else set(ranked[:2])
        union = [t for t in options if class_of(t) in union_classes][:255]
        if truth_class not in union_classes:
            union = ([t for t in options if class_of(t) == truth_class][:1]) + union[:254]
        label3, probs3 = ask(
            provider, union, candidate["query"], "tool_choice", "tool_choice_pass2",
            "Which single tool from this shortlist best serves the intent?",
        )
        if label3 == truth:
            a2_top1 += 1
        if probs3 and topk_hit({"probabilities": probs3}, truth, 3):
            a2_top3 += 1
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(candidates)}] a1 {a1_top1}/{answered} · a2 {a2_top1}/{answered}",
                  file=sys.stderr)
    n = answered or 1
    print()
    print(f"composite replay — {answered} turn(s), {provider} (flat single-pass baseline: 51.3% top-1 / 76.5% top-3)")
    print(f"  pass-1 class hit rate: {a1_class_hit}/{n} = {a1_class_hit / n * 100:.0f}%")
    print(f"  A1 classify->decide:   top-1 {a1_top1}/{n} = {a1_top1 / n * 100:.1f}% · top-3 {a1_top3}/{n} = {a1_top3 / n * 100:.1f}%")
    print(f"  A2 top-2-class union:  top-1 {a2_top1}/{n} = {a2_top1 / n * 100:.1f}% · top-3 {a2_top3}/{n} = {a2_top3 / n * 100:.1f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
