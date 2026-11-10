#!/usr/bin/env python3
"""The verifier measurement (C4's re-open test): can a decision head
distinguish a corrupted tool call from the recorded original?

The corpus is mechanical: recorded successful calls, each corrupted two
ways — argument-level (drop a field / swap two values / mistype a
value's type) and name-level (a plausible wrong tool from the same
catalog). The original is ground truth by construction; no labels.

Per vendor: the noul head "is this call correct for the intent?" over
original and corrupted; the metrics are the separation AUC and the
at-threshold rates — the numbers that decide act-or-repair.
"""

from __future__ import annotations

import json
import random
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))
from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_noul  # noqa: E402

SAMPLE = 60
REQUIRED_HINTS = {
    "read_file": ["path"], "edit": ["path", "old_string", "new_string"],
    "write_file": ["path", "content"], "bash": ["command"],
    "grep": ["pattern"], "glob": ["pattern"], "list_dir": ["path"],
    "remember": ["name", "body"],
}


def load_calls(sample: int) -> list[dict]:
    """Recorded successful calls with their arguments and the intent."""
    sessions_dir = Path.home() / ".fleet" / "sessions"
    calls: list[dict] = []
    for path in sorted(sessions_dir.glob("*.jsonl")):
        try:
            lines = path.read_text(errors="replace").splitlines()
        except OSError:
            continue
        last_user = ""
        pending: dict[str, tuple[str, str]] = {}
        declared: list[str] = []
        for line in lines:
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            kind = record.get("type")
            if kind == "declarations" and not declared:
                declared = record.get("names") or []
            elif kind == "user":
                last_user = (record.get("content") or "")[:2000]
            elif kind == "assistant":
                for call in record.get("tool_calls") or []:
                    fn = call.get("function", {})
                    pending[call.get("id", "")] = (fn.get("name", ""), fn.get("arguments", "{}"))
            elif kind == "tool_result":
                call_id = record.get("call_id", "")
                pair = pending.pop(call_id, None)
                if pair is None:
                    continue
                name, arguments = pair
                output = record.get("output") or ""
                if output.startswith(("error", "denied")):
                    continue
                try:
                    args = json.loads(arguments)
                except json.JSONDecodeError:
                    continue
                if not isinstance(args, dict) or not args:
                    continue
                calls.append({
                    "query": last_user,
                    "tool": name,
                    "arguments": args,
                    "catalog": declared,
                })
    if len(calls) <= sample:
        return calls
    stride = len(calls) / sample
    return [calls[int(i * stride)] for i in range(sample)]


def corrupt_arguments(args: dict, rng: random.Random) -> dict:
    """One argument-level corruption: drop a field, swap two values, or
    mistype a value."""
    keys = list(args.keys())
    mode = rng.choice(["drop", "swap", "mistype"])
    broken = dict(args)
    if mode == "drop" and len(keys) > 1:
        droppable = [k for k in keys if k not in REQUIRED_HINTS.get("", [])]
        victim = rng.choice(keys)
        del broken[victim]
        return broken
    if mode == "swap" and len(keys) > 1:
        a, b = rng.sample(keys, 2)
        broken[a], broken[b] = args[b], args[a]
        return broken
    # mistype: a string value becomes an int, or vice versa
    for key in rng.sample(keys, len(keys)):
        value = args[key]
        if isinstance(value, str) and value.isdigit():
            broken[key] = int(value)
            return broken
        if isinstance(value, int):
            broken[key] = str(value)
            return broken
        if isinstance(value, str):
            broken[key] = value[::-1]
            return broken
    return broken


def corrupt_name(call: dict, rng: random.Random) -> str:
    """A plausible wrong tool from the same catalog."""
    catalog = [t for t in call["catalog"] if t and t != call["tool"]]
    return rng.choice(catalog) if catalog else call["tool"] + "-typo"


def main() -> int:
    provider = sys.argv[1] if len(sys.argv) > 1 else "glide"
    rng = random.Random(20261110)
    calls = load_calls(SAMPLE)
    print(f"{len(calls)} recorded call(s), provider {provider}: original vs corrupted",
          file=sys.stderr)
    task = Task(
        id="call_verify",
        heads=[make_noul(
            "call_verify",
            id="correct",
            yes_label="correct",
            no_label="broken",
            descriptions={
                "correct": "The tool and its arguments are well-formed and serve the intent.",
                "broken": "The call is malformed: a missing or mistyped field, swapped values, or a tool that cannot serve the intent.",
            },
        )],
        description="Is this tool call correct for the intent?",
    )
    pairs = []  # (kind, noul_original, noul_corrupted)
    answered = 0
    for index, call in enumerate(calls):
        intent = call["query"]
        rendered = json.dumps(call["arguments"])
        corrupted_args = corrupt_arguments(call["arguments"], rng)
        wrong_name = corrupt_name(call, rng)
        variants = [
            ("args", json.dumps(corrupted_args)),
            ("name", json.dumps({**call["arguments"], "__tool__": wrong_name})),
        ]
        noul_original = None
        noul_corrupted = None
        kind = "args"
        try:
            a_orig = classify(provider=provider, task=task,
                              text=f"intent:\n{intent}\n\ncall: {call['tool']} {rendered}").head("correct", provider)
            noul_original = (a_orig or {}).get("noul")
            a_corr = classify(provider=provider, task=task,
                              text=f"intent:\n{intent}\n\ncall: {call['tool']} {variants[0][1]}").head("correct", provider)
            noul_corrupted = (a_corr or {}).get("noul")
        except Exception as error:  # noqa: BLE001
            print(f"  [{index}] {type(error).__name__}: {error}", file=sys.stderr)
            continue
        if isinstance(noul_original, (int, float)) and isinstance(noul_corrupted, (int, float)):
            answered += 1
            pairs.append((noul_original, noul_corrupted))
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(calls)}] answered {answered}", file=sys.stderr)
    # separation AUC: P(noul(original) > noul(corrupted))
    wins = ties = 0
    for orig, corr in pairs:
        if orig > corr:
            wins += 1
        elif orig == corr:
            ties += 1
    total = len(pairs) or 1
    auc = (wins + ties / 2) / total
    # at-threshold: original accepted (>= .5), corrupted rejected (< .5)
    orig_ok = sum(1 for o, _ in pairs if o >= 0.5)
    corr_rej = sum(1 for _, c in pairs if c < 0.5)
    both = sum(1 for o, c in pairs if o >= 0.5 and c < 0.5)
    print()
    print(f"verifier — {answered} pair(s), {provider}")
    print(f"  separation AUC (original > corrupted): {auc:.3f}")
    print(f"  original accepted at 0.5:  {orig_ok}/{total} = {orig_ok / total * 100:.1f}%")
    print(f"  corrupted rejected at 0.5: {corr_rej}/{total} = {corr_rej / total * 100:.1f}%")
    print(f"  pairs fully separated (act/repair decided): {both}/{total} = {both / total * 100:.1f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
