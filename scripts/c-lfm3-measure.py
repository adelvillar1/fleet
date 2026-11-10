#!/usr/bin/env python3
"""LFM3-in-the-middle: the generative small model as a list-wise stage.

  L1  glide flat weed -> LFM3 list-wise re-rank of the top-5 (it reads
      the intent plus each survivor's description — the heads only ever
      saw names)
  L2  LFM3 as the weed itself — a schema-aware shortlist from the full
      catalog — graded directly, and composed with glide's noul verify

Ground truth: the recorded choice, exactly as C1.
"""

from __future__ import annotations

import importlib.util
import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "services/sys1/src"))
from sys1.api import classify  # noqa: E402
from sys1.types import Task, make_noul  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "selection_headtohead", REPO / "scripts" / "selection-headtohead.py"
)
_h2h = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_h2h)
load_candidates = _h2h.load_candidates

TOOL_DESCRIPTIONS = {
    "read_file": "Read a file (windowed)",
    "edit": "Edit a file's exact text",
    "write_file": "Write a file",
    "list_dir": "List a directory",
    "grep": "Search file contents",
    "glob": "Find files by pattern",
    "bash": "Run a shell command",
    "process": "Manage background processes",
    "remember": "Write a durable fact to memory",
    "recall": "Search memory for facts",
    "supersede": "Replace an outdated fact",
    "memories": "List the fact corpus",
    "use_skill": "Load a skill's instructions",
    "skills": "List available skills",
    "subagent": "Delegate work to a child agent",
    "goal": "Show or set the session's standing objective",
    "tool_search": "Search deferred lane tools",
    "doctor": "Run startup checks",
}
LANE_DESCRIPTIONS = {
    "sys1": "the judgment lane's decision-model call",
    "sem1": "the embeddings lane (semantic search, probe)",
    "sdm1": "the tabular lane's classification",
    "gen1": "the media lane (imagine, speak)",
    "blend1": "the composed grounded-answer lane",
}


def describe(tool: str) -> str:
    if tool in TOOL_DESCRIPTIONS:
        return TOOL_DESCRIPTIONS[tool]
    if tool.startswith("mcp__"):
        parts = tool.split("__")
        lane = parts[1] if len(parts) > 1 else "?"
        own = parts[2] if len(parts) > 2 else tool
        return f"{LANE_DESCRIPTIONS.get(lane, f'the {lane} lane')} ({own})"
    return tool


for line in open(os.path.expanduser("~/.config/dev-decisions/env")):
    line = line.strip()
    if line and not line.startswith("#") and "=" in line:
        key, _, value = line.partition("=")
        os.environ.setdefault(key.strip(), value.strip())
LFM3_KEY = os.environ.get("LIQUID_API_KEY", "")


def lfm3_rank(intent: str, tools: list[str]) -> list[str]:
    """One LFM3-1.7B call: list-wise ranking of the given tools."""
    listing = "\n".join(f"- {tool}: {describe(tool)}" for tool in tools)
    body = json.dumps({
        "model": "lfm3-1.7b",
        "messages": [
            {"role": "system", "content": (
                "You rank tools for a coding agent. Given the intent and the tool "
                "list, return ONLY a JSON object: {\"ranking\": [\"tool\", ...]} — "
                "most useful first, using only names from the list."
            )},
            {"role": "user", "content": f"Intent:\n{intent[:2000]}\n\nTools:\n{listing}"},
        ],
        "max_tokens": 1024,
    }).encode()
    req = urllib.request.Request(
        "https://api.liquid.ai/openai/v1/chat/completions",
        data=body,
        headers={"Authorization": f"Bearer {LFM3_KEY}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=90) as resp:
            data = json.loads(resp.read())
    except (urllib.error.URLError, OSError, TimeoutError):
        return []
    text = ((data.get("choices") or [{}])[0].get("message") or {}).get("content") or ""
    text = text.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()
    match = re.search(r'"ranking"\s*:\s*\[(.*?)\]', text, re.S)
    if not match:
        # fallback: the tools in order of first appearance in the text
        return [t for t in tools if t in text]
    names = re.findall(r'"([^"]+)"', match.group(1))
    present = [n for n in names if n in tools]
    rest = [t for t in tools if t not in present]
    return present + rest


def main() -> int:
    candidates = load_candidates(120)
    print(f"{len(candidates)} turn(s): L1 glide-weed -> LFM3 re-rank; L2 LFM3 weed (+ glide noul verify)",
          file=sys.stderr)
    n = l1_top1 = l1_top3 = l2_top1 = l2_top3 = l2c_top1 = 0
    for index, candidate in enumerate(candidates):
        options = candidate["options"][:250]
        truth = candidate["truth"]
        intent = candidate["query"]
        # ── L1: glide flat weed -> LFM3 list-wise re-rank ──
        task1 = Task(
            id="tool_choice_pass",
            heads=[__import__("sys1.types", fromlist=["make_choice"]).make_choice(
                "tool_choice_pass", options, id="tool_choice")],
            description="Which single tool from the catalog best serves the intent?",
        )
        a1 = classify(provider="glide", task=task1, text=intent).head("tool_choice", "glide")
        if not a1 or a1.get("_declined"):
            continue
        n += 1
        probs = a1.get("probabilities") or {}
        top5 = sorted(probs, key=lambda t: probs.get(t, 0), reverse=True)[:5]
        lfm1 = lfm3_rank(intent, top5)
        if lfm1 and lfm1[0] == truth:
            l1_top1 += 1
        if truth in lfm1[:3]:
            l1_top3 += 1
        # ── L2: LFM3 as the weed over the full catalog ──
        lfm2 = lfm3_rank(intent, options)
        if lfm2:
            if lfm2[0] == truth:
                l2_top1 += 1
            if truth in lfm2[:3]:
                l2_top3 += 1
            # composed: LFM3's top-5 -> glide noul verify -> product with LFM3's order
            short = lfm2[:5]
            heads = [
                make_noul("survivor_verify", id=f"v_{i}",
                          yes_label=f"required: {tool}", no_label="not the tool")
                for i, tool in enumerate(short)
            ]
            task2 = Task(id="survivor_verify", heads=heads,
                         description="For each shortlisted tool, judge whether the intent requires it.")
            a2 = classify(provider="glide", task=task2, text=intent)
            noul = {}
            for i, tool in enumerate(short):
                slot = a2.head(f"v_{i}", "glide") or {}
                p = slot.get("noul")
                if isinstance(p, (int, float)):
                    noul[tool] = p
            lfm_pos = {tool: len(short) - i for i, tool in enumerate(short)}
            composed = sorted(short, key=lambda t: lfm_pos.get(t, 0) * noul.get(t, 0.0), reverse=True)
            if composed and composed[0] == truth:
                l2c_top1 += 1
        if (index + 1) % 20 == 0:
            print(f"  [{index + 1}/{len(candidates)}] L1 {l1_top1}/{n} · L2 {l2_top1}/{n}", file=sys.stderr)
    print()
    print(f"lfm3-in-the-middle — {n} turn(s) (flat glide: 51.3% top-1 / 76.5% top-3)")
    print(f"  L1 glide-weed -> LFM3 list-wise re-rank:  top-1 {l1_top1}/{n} = {l1_top1 / max(n,1) * 100:.1f}% · top-3 {l1_top3}/{n} = {l1_top3 / max(n,1) * 100:.1f}%")
    print(f"  L2 LFM3 schema-aware weed (direct):       top-1 {l2_top1}/{n} = {l2_top1 / max(n,1) * 100:.1f}% · top-3 {l2_top3}/{n} = {l2_top3 / max(n,1) * 100:.1f}%")
    print(f"  L2c LFM3 weed -> glide noul product:      top-1 {l2c_top1}/{n} = {l2c_top1 / max(n,1) * 100:.1f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
