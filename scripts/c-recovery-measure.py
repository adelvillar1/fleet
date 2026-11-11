#!/usr/bin/env python3
"""C4's closing measurement: the repair leg's recovery rate on the
corrupted-call corpus vs the error-result loop's measured baseline
(C0: 54.2% of failures recovered via the model's next turn).

The repair author is LFM3-1.7B on Liquid's hosted chat API (the same
wire `src/generate.rs` rides). Schema validation is reimplemented here
for the corpus's tools (required fields + string-ness) — a proxy for
`tools::validate`, flagged as such; the live grounding pass does the
real one.
"""

from __future__ import annotations

import importlib.util
import json
import os
import re
import statistics
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

_spec = importlib.util.spec_from_file_location(
    "c_verifier_measure", REPO / "scripts" / "c-verifier-measure.py"
)
_v = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_v)
load_calls = _v.load_calls
corrupt_arguments = _v.corrupt_arguments

REQUIRED = {
    "read_file": {"path"}, "write_file": {"path", "content"},
    "edit": {"path", "old_string", "new_string"}, "bash": {"command"},
    "grep": {"pattern"}, "glob": {"pattern"}, "list_dir": {"path"},
}


def schema_ok(tool: str, args: dict) -> tuple[bool, str]:
    required = REQUIRED.get(tool.removeprefix("mcp__gen1__").removeprefix("mcp__"), set())
    if tool not in REQUIRED and not tool.startswith("mcp__"):
        required = set()  # unknown tool: presence-and-parse only
    for field in required:
        if field not in args:
            return False, f"missing {field}"
        if not isinstance(args[field], str):
            return False, f"{field} not a string"
    return True, "ok"


for line in open(os.path.expanduser("~/.config/dev-decisions/env")):
    line = line.strip()
    if line and not line.startswith("#") and "=" in line:
        key, _, value = line.partition("=")
        os.environ.setdefault(key.strip(), value.strip())
KEY = os.environ.get("LIQUID_API_KEY", "")


def lfm3_repair(tool: str, raw: str, error: str) -> tuple[str | None, float]:
    body = json.dumps({
        "model": "lfm3-1.7b",
        "max_tokens": 1024,
        "messages": [
            {"role": "system", "content": (
                "You repair a tool call's JSON arguments so they pass the tool's own "
                "validation. Return only the corrected JSON object — no markdown fence, "
                "no commentary. Keep every field's meaning; fix shape, types, and "
                "required fields."
            )},
            {"role": "user", "content":
                f"The `{tool}` call's raw arguments:\n{raw}\n\nThe validation failure:\n{error}\n\nReturn ONLY the corrected JSON object."},
        ],
    }).encode()
    req = urllib.request.Request(
        "https://api.liquid.ai/openai/v1/chat/completions", data=body,
        headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"},
        method="POST",
    )
    start = time.monotonic()
    try:
        with urllib.request.urlopen(req, timeout=90) as resp:
            data = json.loads(resp.read())
    except (urllib.error.URLError, OSError, TimeoutError) as e:
        return None, time.monotonic() - start
    latency = time.monotonic() - start
    text = ((data.get("choices") or [{}])[0].get("message") or {}).get("content") or ""
    text = text.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
    return text, latency


def main() -> int:
    rng = __import__("random").Random(20261110)
    calls = load_calls(60)
    print(f"{len(calls)} recorded call(s): corrupted, then LFM3 repair, then schema check",
          file=sys.stderr)
    recovered = attempted = 0
    latencies = []
    for index, call in enumerate(calls):
        corrupted = corrupt_arguments(call["arguments"], rng)
        required = REQUIRED.get(call["tool"], None)
        if required is None:
            continue  # unknown schema: the guard honestly cannot validate it
        missing = [f for f in required if f not in corrupted]
        if not missing:
            # force a schema break: drop one required field, so every
            # attempt is a genuine validation failure (the corpus the
            # guard actually covers)
            victim = sorted(required)[rng.randrange(len(required))]
            del corrupted[victim]
            missing = [victim]
        raw = json.dumps(corrupted)
        error = f"missing required field: {missing}"
        repaired, latency = lfm3_repair(call["tool"], raw, error)
        attempted += 1
        if repaired is None:
            continue
        latencies.append(latency)
        try:
            parsed = json.loads(repaired)
        except json.JSONDecodeError:
            print(f"  [{index}] unparseable repair", file=sys.stderr)
            continue
        ok, reason = schema_ok(call["tool"], parsed if isinstance(parsed, dict) else {})
        recovered += int(ok)
        if not ok:
            print(f"  [{index}] {call['tool']}: failed schema check ({reason})", file=sys.stderr)
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(calls)}] recovered {recovered}/{attempted}", file=sys.stderr)
    print()
    print(f"recovery comparison — {attempted} corrupted call(s), repair author LFM3-1.7B")
    if latencies:
        print(f"  repair-leg schema-pass recovery: {recovered}/{attempted} = {recovered / attempted * 100:.1f}%")
        print(f"  p50 repair latency: {statistics.median(latencies):.2f}s")
    print(f"  error-result loop baseline (C0, real traffic): 54.2%")
    verdict = "PASSES" if latencies and recovered / attempted > 0.542 else "DOES NOT PASS"
    print(f"  criterion C4's bar (strictly higher than the loop): {verdict}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
