#!/usr/bin/env python3
"""The atomic-decomposition replay: the selection question broken into
atomic decisions, on the same 120 recorded turns as C1.

  B   multi-head relevance fan-out — ONE classify request carrying one
      yes/no head per declared tool ("does serving this intent require
      this tool?"), the argmax the selection; the sharp two-way shape
      C12 measured, batched the way plan-gate batches its heads.

  A2c top-2-class union WITHOUT the truth leak (the clean re-run of the
      contaminated A2).
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))
from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_noul, make_choice  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "selection_headtohead", REPO / "scripts" / "selection-headtohead.py"
)
_h2h = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_h2h)
load_candidates = _h2h.load_candidates
topk_hit = _h2h.topk_hit

_spec2 = importlib.util.spec_from_file_location(
    "c_composite_measure", REPO / "scripts" / "c-composite-measure.py"
)
_c = importlib.util.module_from_spec(_spec2)
_spec2.loader.exec_module(_c)
class_of = _c.class_of


def main() -> int:
    provider = sys.argv[1] if len(sys.argv) > 1 else "glide"
    candidates = load_candidates(120)
    print(f"{len(candidates)} turn(s), provider {provider}: B atomic fan-out, A2c clean union",
          file=sys.stderr)
    b_top1 = b_top3 = a2c_top1 = a2c_top3 = a2c_class_hit = answered = 0
    for index, candidate in enumerate(candidates):
        options = candidate["options"]
        truth = candidate["truth"]
        # ── B: one request, one yes/no head per declared tool ──
        heads = [
            make_noul(
                "tool_relevance",
                id=f"rel_{i}",
                yes_label=f"required: {tool}",
                no_label="not required",
                descriptions={
                    f"required: {tool}": f"Serving this intent requires the {tool} tool.",
                    "not required": "This tool is not what the intent needs.",
                },
            )
            for i, tool in enumerate(options[:250])
        ]
        task = Task(
            id="tool_relevance_fanout",
            heads=heads,
            description="For each tool, judge independently whether the intent requires it.",
        )
        answer = classify(provider=provider, task=task, text=candidate["query"])
        scored = []
        for i, tool in enumerate(options[:250]):
            slot = answer.head(f"rel_{i}", provider) or {}
            p = slot.get("noul")
            if isinstance(p, (int, float)):
                scored.append((p, tool))
        answered += 1
        scored.sort(reverse=True)
        ranked_tools = [tool for _, tool in scored]
        if ranked_tools and ranked_tools[0] == truth:
            b_top1 += 1
        if truth in ranked_tools[:3]:
            b_top3 += 1
        # ── A2c: clean top-2-class union, no truth leak ──
        truth_class = class_of(truth)
        classes = sorted({class_of(tool) for tool in options} | {"other"})
        label1, probs1 = None, {}
        task1 = Task(
            id="tool_class_pass",
            heads=[make_choice("tool_class_pass", classes, id="tool_class")],
            description="Which class of work does this intent need?",
        )
        a1 = classify(provider=provider, task=task1, text=candidate["query"]).head("tool_class", provider)
        if a1 and not a1.get("_declined"):
            label1 = a1.get("label")
            probs1 = a1.get("probabilities") or {}
        ranked = sorted(probs1, key=lambda c: probs1.get(c, 0), reverse=True)
        union_classes = set(ranked[:2])
        a2c_class_hit += int(truth_class in union_classes)
        union = [t for t in options if class_of(t) in union_classes][:250]
        if union:
            task2 = Task(
                id="tool_choice_pass",
                heads=[make_choice("tool_choice_pass", union, id="tool_choice")],
                description="Which single tool from this shortlist best serves the intent?",
            )
            a2 = classify(provider=provider, task=task2, text=candidate["query"]).head("tool_choice", provider)
            if a2 and not a2.get("_declined"):
                if a2.get("label") == truth:
                    a2c_top1 += 1
                if topk_hit(a2, truth, 3):
                    a2c_top3 += 1
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(candidates)}] B {b_top1}/{answered} · A2c {a2c_top1}/{answered}",
                  file=sys.stderr)
    n = answered or 1
    print()
    print(f"atomic replay — {answered} turn(s), {provider} (flat: 51.3% top-1 / 76.5% top-3)")
    print(f"  B  atomic fan-out (1 request, N yes/no heads): top-1 {b_top1}/{n} = {b_top1 / n * 100:.1f}% · top-3 {b_top3}/{n} = {b_top3 / n * 100:.1f}%")
    print(f"  A2c top-2-class union (clean):  top-1 {a2c_top1}/{n} = {a2c_top1 / n * 100:.1f}% · top-3 {a2c_top3}/{n} = {a2c_top3 / n * 100:.1f}% (pass-1 union hit {a2c_class_hit}/{n})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
