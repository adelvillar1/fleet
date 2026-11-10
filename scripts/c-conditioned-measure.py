#!/usr/bin/env python3
"""The owner's conditioned pipeline: ask the question in stages that
keep the full catalog visible.

  Q1  shape of the work (fleet's shape vocabulary)
  Q2  "given this is a {shape} task, which tool is most probable?" —
      the FULL catalog, conditioned, never narrowed
  Q3  noul verify of Q2's winner
  P   the composed decision: Q2 probability x Q3 verification

Plus the score fan-out diagnostic (its 1.7% is below random — either
the parse or the shape is broken, and the raw answers say which).
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))
from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_choice, make_noul, make_score  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "selection_headtohead", REPO / "scripts" / "selection-headtohead.py"
)
_h2h = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_h2h)
load_candidates = _h2h.load_candidates

SHAPES = ["state-sync", "change", "docs", "build", "deploy", "rendering",
          "fail-open", "drops-the-row", "incomplete", "resolved"]


def main() -> int:
    diagnose = "--diagnose-score" in sys.argv
    candidates = load_candidates(6 if diagnose else 120)
    if diagnose:
        options = candidates[0]["options"][:250]
        heads = [
            make_score("tool_score", id=f"sc_{i}", score_min=1, score_max=5,
                       legend=["1 - unrelated", "2 - marginally related", "3 - could plausibly help",
                               "4 - well suited", "5 - exactly right"])
            for i, _ in enumerate(options)
        ]
        task = Task(id="tool_score_fanout", heads=heads,
                    description="For each tool, rate how well it serves the intent.")
        answer = classify(provider="glide", task=task, text=candidates[0]["query"])
        for i, tool in enumerate(options[:6]):
            print(f"DIAG {tool}: {answer.head(f'sc_{i}', 'glide')}")
        return 0
    provider = "glide"
    n = 0
    q1_hit = 0
    q2_top1 = q2_top3 = 0
    p_top1 = p_top3 = 0
    v_confirm = 0
    for index, candidate in enumerate(candidates):
        options = candidate["options"][:250]
        truth = candidate["truth"]
        intent = candidate["query"]
        # Q1: the shape
        task1 = Task(
            id="task_shape",
            heads=[make_choice("task_shape", SHAPES, id="shape")],
            description="What shape of work is this intent?",
        )
        a1 = classify(provider=provider, task=task1, text=intent).head("shape", provider)
        shape = (a1 or {}).get("label", "general work")
        # Q2: shape-conditioned choice over the FULL catalog
        task2 = Task(
            id="tool_choice_conditioned",
            heads=[make_choice("tool_choice_conditioned", options, id="tool_choice")],
            description=(
                f"This is a {shape} task. Which single tool from the catalog has "
                f"the highest probability of serving it?"
            ),
        )
        a2 = classify(provider=provider, task=task2, text=intent).head("tool_choice", provider)
        if not a2 or a2.get("_declined"):
            continue
        n += 1
        probs = a2.get("probabilities") or {}
        ranked = sorted(probs, key=lambda t: probs.get(t, 0), reverse=True)
        winner = ranked[0] if ranked else None
        if winner == truth:
            q2_top1 += 1
        if truth in ranked[:3]:
            q2_top3 += 1
        # Q3: noul verify of the winner
        if winner:
            task3 = Task(
                id="winner_verify",
                heads=[make_noul("winner_verify", id="v",
                                 yes_label=f"required: {winner}",
                                 no_label="not the tool",
                                 descriptions={
                                     f"required: {winner}": f"The {winner} tool is what this {shape} intent needs.",
                                     "not the tool": "This is not what the intent needs.",
                                 })],
                description=f"Is the {winner} tool the right one for this {shape} intent?",
            )
            a3 = classify(provider=provider, task=task3, text=intent).head("v", provider)
            noul = (a3 or {}).get("noul")
            if isinstance(noul, (int, float)):
                if winner == truth:
                    v_confirm += int(noul >= 0.5)
                # the composed decision: Q2 probability x Q3 verification
                scored = sorted(probs.items(),
                                key=lambda kv: kv[1] * (noul if kv[0] == winner else (1 - noul) * 0.3),
                                reverse=True)
                if scored and scored[0][0] == truth:
                    p_top1 += 1
                if truth in [t for t, _ in scored[:3]]:
                    p_top3 += 1
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(candidates)}] Q2 {q2_top1}/{n} · P {p_top1}/{n}", file=sys.stderr)
    print()
    print(f"conditioned pipeline — {n} turn(s), {provider} (flat glide: 51.3% top-1 / 76.5% top-3)")
    print(f"  Q1 shape asked (no direct ground truth; the pipeline judges end to end)")
    print(f"  Q2 shape-conditioned full-catalog choice:  top-1 {q2_top1}/{n} = {q2_top1 / max(n,1) * 100:.1f}% · top-3 {q2_top3}/{n} = {q2_top3 / max(n,1) * 100:.1f}%")
    print(f"  P  Q2 x Q3 verification composed:          top-1 {p_top1}/{n} = {p_top1 / max(n,1) * 100:.1f}% · top-3 {p_top3}/{n} = {p_top3 / max(n,1) * 100:.1f}%")
    print(f"  (the noul confirmed the winner in {v_confirm} turn(s) where the winner was right)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
