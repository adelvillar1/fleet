# Generation — the hosted small-model text leg

The grounded-selection plan's Phase 4 feature doc: the seam through which
prose a small hosted model authors rides, what the fallbacks are, and
which surfaces are owner-visible. Status mirrors
`docs/plans/2026-11-10-grounded-selection.md`.

## The seam

One wire, one key: blend1's `generate` MCP tool
(`services/blend1/service/mcp_server.py`) wrapping the proven Liquid chat
lane (`services/blend1/src/blend1/lanes.py`). Probed live 2026-11-10:

- `GET https://api.liquid.ai/v1/chat/models` serves five models; the text
  legs are **lfm3-1.7b** (the default — small, fast, cheap) and
  **lfm2.5-24b-a2b** (the synthesis lane's proven model). Vision, audio,
  and omni are served but are not text legs.
- Both text models **reason**: reasoning tokens consume the completion
  budget, and a budget below the model's reasoning needs returns
  `content=null` with `finish_reason=length` (measured: 64 tokens died
  entirely inside lfm3-1.7b's reasoning). The leg clamps the budget to a
  **1024-token floor** server-side — the caller's smaller number never
  reaches the wire.

Fleet's side is `src/generate.rs`: one bounded call (spawn, handshake,
`generate` declared, call, teardown — 120s budget), the leg's own JSON
shape parsed (`status`/`text`/`model`), every failure a named `Err`. The
caller owns the fallback; a dead lane never takes a run down with it.

## Who rides it, and what the fallback is

| Surface | Lane behavior | Fallback (today's path) |
|---|---|---|
| **Compaction fold** (`maybe_compact`) | The replaced transcript rides role-named prose with the compaction instruction as the system prompt; the lane's summary replaces the conversation. Lane tokens never enter the routed window estimate. | The routed fold request — named on the progress line when the lane was unavailable. |
| **End-of-run title** | After a main-surface run finishes (owner sessions only — scheduled, probe, and subagent traffic never spends), the lane names the session from the task. | The deterministic ladder (owner → goal → plan → first message), which keeps serving. |
| **End-of-run archive summary** | One sentence for the archive card, from the task plus the final message. | The head-and-tail heuristic (`session::summarize`). |

The owner's own title always wins: an existing `sessions-titles.json`
entry is never overwritten, and an owner-named session takes one lane
call (the summary) instead of two.

Both sidecars live beside the logs — `sessions-titles.json` (existing)
and `sessions-summaries.json` (new) — written temp+rename, entries dropped
when their log is gone. The face's sessions listing renders `lane_summary`
over the head-and-tail `first_text` when present.

The act is gated by `AgentOptions.lane_prose` — the same actor
`health_note` names: the CLI and the face's run paths set it; children,
eval runs, and tests leave it off, so a test's premise never rides the
machine's lanes.

## What this seam deliberately does not do

- It never authors a turn: generation here is prose the caller asked for
  (fold, title, summary), not the model's own next step.
- It never touches the routed window estimate: lane tokens are blend1's
  spend, named on the progress line, and `self.usage` stays the routed
  API's truth.
- It never spends on machine traffic: scheduled runs, probes, and
  subagents carry `lane_prose: false`.
