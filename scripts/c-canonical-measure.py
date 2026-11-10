#!/usr/bin/env python3
"""The canonical-framing re-run: the vendor's own routing recipe applied
to fleet's tool selection.

Corrections vs the C1 harness (all four documented):
  1. every choice label carries its criteria description
  2. an explicit `no_tool` escape route (the list need not be exhaustive)
  3. state names the task and the routing decision explicitly
  4. glide-native confidence (the margin) read from the answer

Same 120 recorded turns, same ground truth.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))
from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_choice  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "selection_headtohead", REPO / "scripts" / "selection-headtohead.py"
)
_h2h = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_h2h)
load_candidates = _h2h.load_candidates

sys.path.insert(0, str(REPO / "scripts"))
_spec2 = importlib.util.spec_from_file_location(
    "c_lfm3_measure", REPO / "scripts" / "c-lfm3-measure.py"
)
_l = importlib.util.module_from_spec(_spec2)
_spec2.loader.exec_module(_l)
describe = _l.describe

NO_TOOL = "no_tool"
NO_TOOL_DESC = "None of the listed tools fit this task; it needs something else entirely."


def main() -> int:
    provider = sys.argv[1] if len(sys.argv) > 1 else "glide"
    candidates = load_candidates(120)
    print(f"{len(candidates)} turn(s), provider {provider}: canonical framing "
          f"(described criteria + escape route)", file=sys.stderr)
    n = top1 = top3 = escape_honest = 0
    margins_right: list[float] = []
    margins_wrong: list[float] = []
    for index, candidate in enumerate(candidates):
        options = candidate["options"][:250]
        truth = candidate["truth"]
        criteria = {tool: describe(tool) for tool in options}
        criteria[NO_TOOL] = NO_TOOL_DESC
        task = Task(
            id="tool_routing",
            heads=[make_choice(
                "tool_routing",
                list(criteria.keys()),
                id="destination",
                descriptions=criteria,
            )],
            description=(
                "Which single tool should serve this task? If none fit, choose "
                f"{NO_TOOL}."
            ),
        )
        state = (
            f"task: {candidate['query'][:4000]}\n\n"
            "routing decision: pick the one tool that best serves the task."
        )
        answer = classify(provider=provider, task=task, text=state).head("destination", provider)
        if not answer or answer.get("_declined"):
            continue
        n += 1
        probs = answer.get("probabilities") or {}
        ranked = sorted(probs, key=lambda t: probs.get(t, 0), reverse=True)
        pick = answer.get("choice") or (ranked[0] if ranked else None)
        margin = answer.get("confidence")
        if pick == NO_TOOL:
            # the escape route: honest only if the truth really is unreachable
            pass
        else:
            if pick == truth:
                top1 += 1
                if isinstance(margin, (int, float)):
                    margins_right.append(margin)
            if truth in ranked[:3]:
                top3 += 1
            if isinstance(margin, (int, float)):
                margins_wrong.append(margin)
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(candidates)}] {top1}/{n}", file=sys.stderr)
    print()
    print(f"canonical framing — {n} turn(s), {provider}")
    print(f"  bare-labels flat baseline: 51.3% top-1 / 76.5% top-3 (glide) · 56.7% / 75.8% (drex)")
    print(f"  canonical (described criteria + escape route): top-1 {top1}/{n} = {top1 / max(n,1) * 100:.1f}% · top-3 {top3}/{n} = {top3 / max(n,1) * 100:.1f}%")
    def auc(pos, neg):
        if not pos or not neg:
            return None
        wins = sum(1 for m in pos for n2 in neg if m > n2)
        ties = sum(1 for m in pos for n2 in neg if m == n2)
        return (wins + ties / 2) / (len(pos) * len(neg))
    area = auc(margins_right, margins_wrong)
    print(f"  glide-native margin AUC (right vs wrong): {'n/a' if area is None else f'{area:.3f}'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
