#!/usr/bin/env python3
"""C1 — the offline head-to-head over recorded selection turns (no runtime change).

Replays recorded tool calls from the session archive: every call that
resolved `exact`/`alias` and succeeded is a ground-truth selection. The
question put to each head: given the intent the turn rode on, which tool
from that session's own declared catalog? Grade per head: top-1 accuracy
against the recorded choice, and the margin's AUC for predicting its own
correctness — the ambiguity signal C5/C6 build on.

Providers: glide, drex, liquid — the three decision families already on
the keychain. Deterministic sampling (a fixed stride over the archive in
name order), so re-runs are comparable.

Run:  python3 scripts/selection-headtohead.py [--sample N] [--providers p1,p2]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))

from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_choice  # noqa: E402

GLIDE_CHOICE_CAP = 255
MAX_QUERY_CHARS = 4_000


def load_candidates(sample: int) -> list[dict]:
    """Walk the archive in name order; stride-sample to `sample` entries."""
    sessions_dir = Path.home() / ".fleet" / "sessions"
    candidates: list[dict] = []
    broken_logs = 0
    for path in sorted(sessions_dir.glob("*.jsonl")):
        declared: list[str] | None = None
        last_user = ""
        last_assistant = ""
        pending: dict[str, str] = {}  # call_id -> resolved name
        try:
            lines = path.read_text(errors="replace").splitlines()
        except OSError:
            broken_logs += 1
            continue
        for line in lines:
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue  # torn tail or junk line: skip, keep reading
            kind = record.get("type")
            if kind == "declarations" and declared is None:
                names = record.get("names") or []
                if names:
                    declared = names
            elif kind == "user":
                last_user = (record.get("content") or "")[:MAX_QUERY_CHARS]
            elif kind == "assistant":
                last_assistant = (record.get("content") or "")[:MAX_QUERY_CHARS]
                for call in record.get("tool_calls") or []:
                    pass  # calls arrive whole in the resolution records
            elif kind == "tool_resolution":
                if record.get("method") in ("exact", "alias"):
                    pending[record.get("call_id", "")] = record.get("resolved", "")
            elif kind == "tool_result":
                call_id = record.get("call_id", "")
                truth = pending.pop(call_id, None)
                if truth is None:
                    continue
                output = record.get("output") or ""
                if output.startswith(("error", "denied")):
                    continue  # a failed call is not a ground-truth choice
                if not declared:
                    continue
                query = last_user.strip()
                if last_assistant.strip():
                    query = f"{query}\n\n(the assistant's step so far: {last_assistant.strip()})"
                candidates.append(
                    {
                        "session": path.stem,
                        "query": query,
                        "truth": truth,
                        "options": declared,
                    }
                )
    if broken_logs:
        print(f"skipped {broken_logs} unreadable log(s)", file=sys.stderr)
    if len(candidates) <= sample:
        return candidates
    stride = len(candidates) / sample
    return [candidates[int(i * stride)] for i in range(sample)]


def clamp_options(options: list[str], truth: str) -> list[str]:
    """GLiDE's choice head takes at most 255 options; the truth always rides."""
    if len(options) <= GLIDE_CHOICE_CAP:
        return options
    keep = [truth] if truth in options else []
    keep += sorted(set(options) - {truth})[: GLIDE_CHOICE_CAP - len(keep)]
    return keep


def margin_of(answer: dict) -> float | None:
    """Top-minus-second of the head's own probability distribution."""
    probs = [v for v in (answer.get("probabilities") or {}).values() if isinstance(v, (int, float))]
    if len(probs) < 2:
        return answer.get("confidence") if isinstance(answer.get("confidence"), (int, float)) else None
    probs.sort(reverse=True)
    return probs[0] - probs[1]


def auc(pairs: list[tuple[float, bool]]) -> float | None:
    """Rank-based AUC (Mann-Whitney): P(margin correct > margin wrong)."""
    pos = sorted(m for m, ok in pairs if ok)
    neg = sorted(m for m, ok in pairs if not ok)
    if not pos or not neg:
        return None
    wins = ties = 0
    for m in pos:
        for n in neg:
            if m > n:
                wins += 1
            elif m == n:
                ties += 1
    return (wins + ties / 2) / (len(pos) * len(neg))


def topk_hit(answer: dict, truth: str, k: int = 3) -> bool:
    """The truth among the head's top-k probabilities — the injection
    role's number: the head shortlists, the model picks."""
    probs = answer.get("probabilities") or {}
    ranked = sorted(
        (label for label, p in probs.items() if isinstance(p, (int, float))),
        key=lambda label: probs[label],
        reverse=True,
    )
    return truth in ranked[:k]


def run(providers: list[str], candidates: list[dict]) -> dict[str, dict]:
    results: dict[str, dict] = {
        p: {"answered": 0, "errors": 0, "correct": 0, "top3": 0, "pairs": []}
        for p in providers
    }
    total = len(candidates) * len(providers)
    done = 0
    for candidate in candidates:
        options = clamp_options(candidate["options"], candidate["truth"])
        task = Task(
            id="tool_select",
            heads=[
                make_choice(
                    "tool_select",
                    options,
                    id="tool_choice",
                    descriptions=None,
                )
            ],
            description="Given the owner's intent, which single tool from the catalog best serves the next step?",
        )
        for provider in providers:
            done += 1
            slot = results[provider]
            try:
                answer = classify(
                    provider=provider,
                    task=task,
                    text=candidate["query"],
                ).head("tool_choice", provider)
            except Exception as error:  # noqa: BLE001 — one dead call is one data point lost
                slot["errors"] += 1
                print(f"  [{done}/{total}] {provider}: {type(error).__name__}: {error}", file=sys.stderr)
                continue
            if not answer or answer.get("_declined"):
                slot["errors"] += 1
                continue
            slot["answered"] += 1
            correct = answer.get("label") == candidate["truth"]
            slot["correct"] += int(correct)
            slot["top3"] += int(topk_hit(answer, candidate["truth"]))
            margin = margin_of(answer)
            if margin is not None:
                slot["pairs"].append((margin, correct))
            if done % 30 == 0:
                print(f"  [{done}/{total}] answered so far: " + ", ".join(
                    f"{p} {results[p]['correct']}/{results[p]['answered']}" for p in providers
                ), file=sys.stderr)
    return results


def report(results: dict[str, dict], candidates: list[dict]) -> str:
    lines = []
    lines.append(f"C1 head-to-head — {len(candidates)} recorded selection turn(s), "
                 f"each asked of every head over that session's own declared catalog")
    lines.append("")
    lines.append("| head | answered | top-1 | top-3 hit | accuracy | margin AUC | errors |")
    lines.append("|---|---|---|---|---|---|---|")
    for provider, slot in results.items():
        answered = slot["answered"]
        accuracy = slot["correct"] / answered if answered else 0.0
        top3 = slot["top3"] / answered if answered else 0.0
        area = auc(slot["pairs"])
        lines.append(
            f"| {provider} | {answered} | {slot['correct']} | {top3 * 100:.1f}% | {accuracy * 100:.1f}% | "
            f"{'n/a' if area is None else f'{area:.3f}'} | {slot['errors']} |"
        )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sample", type=int, default=120)
    parser.add_argument("--providers", default="glide,drex,liquid")
    args = parser.parse_args()
    providers = [p.strip() for p in args.providers.split(",") if p.strip()]
    print(f"walking the archive for up to {args.sample} recorded selection turn(s)…", file=sys.stderr)
    candidates = load_candidates(args.sample)
    if not candidates:
        print("no recorded selection turns found", file=sys.stderr)
        return 1
    print(f"{len(candidates)} candidate(s); asking {', '.join(providers)}…", file=sys.stderr)
    results = run(providers, candidates)
    print(report(results, candidates))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
