#!/usr/bin/env python3
"""The score fan-out (the owner's reversed question): one request, one
ordinal score head per declared tool — "how well does this tool serve
this atomic task, 1-5" — ranked by score.

  S   score fan-out over the full catalog (the direct answer)
  S5  the score as a re-rank of glide's flat top-5 (mirrors the
      weed-then-verify but with graded scores instead of binary noul)

Ground truth: the recorded choice, exactly as C1.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))
from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_choice, make_score  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "selection_headtohead", REPO / "scripts" / "selection-headtohead.py"
)
_h2h = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_h2h)
load_candidates = _h2h.load_candidates

LEGEND = [
    "1 - unrelated to the intent",
    "2 - marginally related",
    "3 - could plausibly help",
    "4 - well suited to the intent",
    "5 - exactly what the intent needs",
]


def score_of(slot: dict) -> float | None:
    if not slot:
        return None
    for key in ("score", "noul"):
        value = slot.get(key)
        if isinstance(value, (int, float)):
            return float(value)
    return None


def main() -> int:
    provider = sys.argv[1] if len(sys.argv) > 1 else "glide"
    candidates = load_candidates(120)
    print(f"{len(candidates)} turn(s), provider {provider}: S score fan-out, S5 score re-rank of glide's top-5",
          file=sys.stderr)
    n = s_top1 = s_top3 = s5_top1 = s5_top3 = 0
    for index, candidate in enumerate(candidates):
        options = candidate["options"][:250]
        truth = candidate["truth"]
        intent = candidate["query"]
        # ── S: one request, one score head per tool ──
        heads = [
            make_score("tool_score", id=f"sc_{i}", score_min=1, score_max=5, legend=LEGEND)
            for i, _ in enumerate(options)
        ]
        task = Task(
            id="tool_score_fanout",
            heads=heads,
            description="For each tool, rate how well it serves the intent on the 1-5 scale.",
        )
        answer = classify(provider=provider, task=task, text=intent)
        scored = []
        for i, tool in enumerate(options):
            value = score_of(answer.head(f"sc_{i}", provider) or {})
            if value is not None:
                scored.append((value, tool))
        answered = bool(scored)
        if answered:
            n += 1
        scored.sort(reverse=True)
        ranked = [tool for _, tool in scored]
        if ranked and ranked[0] == truth:
            s_top1 += 1
        if truth in ranked[:3]:
            s_top3 += 1
        # ── S5: glide's flat top-5, re-ranked by score ──
        task1 = Task(
            id="tool_choice_pass",
            heads=[make_choice("tool_choice_pass", options, id="tool_choice")],
            description="Which single tool from the catalog best serves the intent?",
        )
        a1 = classify(provider="glide", task=task1, text=intent).head("tool_choice", "glide")
        if a1 and not a1.get("_declined"):
            probs = a1.get("probabilities") or {}
            top5 = sorted(probs, key=lambda t: probs.get(t, 0), reverse=True)[:5]
            heads5 = [
                make_score("tool_score", id=f"s5_{i}", score_min=1, score_max=5, legend=LEGEND)
                for i, _ in enumerate(top5)
            ]
            task5 = Task(
                id="tool_score_survivors",
                heads=heads5,
                description="For each shortlisted tool, rate how well it serves the intent.",
            )
            a5 = classify(provider=provider, task=task5, text=intent)
            rescored = []
            for i, tool in enumerate(top5):
                value = score_of(a5.head(f"s5_{i}", provider) or {})
                if value is not None:
                    rescored.append((value, tool))
            rescored.sort(reverse=True)
            rank5 = [tool for _, tool in rescored]
            if rank5 and rank5[0] == truth:
                s5_top1 += 1
            if truth in rank5[:3]:
                s5_top3 += 1
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(candidates)}] S {s_top1}/{n} · S5 {s5_top1}/{n}", file=sys.stderr)
    print()
    print(f"score fan-out — {n} turn(s), {provider} (flat glide: 51.3% top-1 / 76.5% top-3)")
    print(f"  S  score fan-out over the catalog:      top-1 {s_top1}/{n} = {s_top1 / max(n,1) * 100:.1f}% · top-3 {s_top3}/{n} = {s_top3 / max(n,1) * 100:.1f}%")
    print(f"  S5 score re-rank of glide's top-5:      top-1 {s5_top1}/{n} = {s5_top1 / max(n,1) * 100:.1f}% · top-3 {s5_top3}/{n} = {s5_top3 / max(n,1) * 100:.1f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
