#!/usr/bin/env python3
"""The weed-then-verify replay (the owner's shape): one flat glide pass
weeds the catalog to its top-k, then one batched noul request verifies
each survivor independently, and the re-rank comes from the verification.

Variants, all from the same two requests per turn:
  noul-only      survivors re-ranked by their yes-probability
  product        flat probability x noul probability
  flat-only k=5  the weed's own ordering (the control: does the noul
                 re-rank beat the weed that fed it?)
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))
from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_choice, make_noul  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "selection_headtohead", REPO / "scripts" / "selection-headtohead.py"
)
_h2h = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_h2h)
load_candidates = _h2h.load_candidates

K = 5


def main() -> int:
    args = sys.argv[1:]
    provider = args[0] if args else "glide"
    verify_provider = args[1] if len(args) > 1 else provider
    candidates = load_candidates(120)
    print(f"{len(candidates)} turn(s), provider {provider}: flat weed to top-{K}, noul verify, re-rank",
          file=sys.stderr)
    n = 0
    # [variant][metric]
    score = {v: {"top1": 0, "top3": 0} for v in
             ("noul3", "noul5", "product", "flat5", "flat1base")}
    for index, candidate in enumerate(candidates):
        options = candidate["options"][:250]
        truth = candidate["truth"]
        # pass 1: the weed
        task1 = Task(
            id="tool_choice_pass",
            heads=[make_choice("tool_choice_pass", options, id="tool_choice")],
            description="Which single tool from the catalog best serves the intent?",
        )
        a1 = classify(provider=provider, task=task1, text=candidate["query"]).head("tool_choice", provider)
        if not a1 or a1.get("_declined"):
            continue
        n += 1
        probs = a1.get("probabilities") or {}
        ranked = sorted(probs, key=lambda t: probs.get(t, 0), reverse=True)
        top5 = ranked[:K]
        if ranked and ranked[0] == truth:
            score["flat1base"]["top1"] += 1
        if truth in top5:
            score["flat5"]["top1"] += int(ranked.index(truth) == 0)
            score["flat5"]["top3"] += int(ranked.index(truth) < 3)
        # pass 2: one batched noul request over the survivors
        heads = [
            make_noul(
                "survivor_verify",
                id=f"v_{i}",
                yes_label=f"required: {tool}",
                no_label="not the tool",
                descriptions={
                    f"required: {tool}": f"Serving this intent requires the {tool} tool.",
                    "not the tool": "This is not what the intent needs.",
                },
            )
            for i, tool in enumerate(top5)
        ]
        task2 = Task(
            id="survivor_verify",
            heads=heads,
            description="For each shortlisted tool, judge independently whether the intent requires it.",
        )
        a2 = classify(provider=verify_provider, task=task2, text=candidate["query"])
        noul = {}
        for i, tool in enumerate(top5):
            slot = a2.head(f"v_{i}", provider) or {}
            p = slot.get("noul")
            if isinstance(p, (int, float)):
                noul[tool] = p
        # variants over the survivor set
        noul_ranked = sorted(top5, key=lambda t: noul.get(t, 0.0), reverse=True)
        prod_ranked = sorted(top5, key=lambda t: probs.get(t, 0.0) * noul.get(t, 0.0), reverse=True)
        for k, ranked_list in (("noul5", noul_ranked), ("product", prod_ranked), ("noul3", noul_ranked)):
            cut = ranked_list[:3] if k == "noul3" else ranked_list
            if cut and cut[0] == truth:
                score[k]["top1"] += 1
            if truth in cut[:3]:
                score[k]["top3"] += 1
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(candidates)}] noul5 {score['noul5']['top1']}/{n}",
                  file=sys.stderr)
    print()
    print(f"weed-then-verify — {n} turn(s), weed={provider} verify={verify_provider} (flat single-pass: 51.3% top-1 / 76.5% top-3)")
    for variant, label in (
        ("flat1base", "flat top-1 (the weed's own pick)"),
        ("flat5", "flat ordering of the top-5 (control)"),
        ("noul5", "noul re-rank of the top-5"),
        ("noul3", "noul re-rank of the top-3"),
        ("product", "flat x noul product re-rank"),
    ):
        d = score[variant]
        print(f"  {label:42} top-1 {d['top1']}/{n} = {d['top1'] / n * 100:.1f}% · top-3 {d['top3']}/{n} = {d['top3'] / n * 100:.1f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
