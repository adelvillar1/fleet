---
name: ollama-cloud-roster-expanded
description: "2026-10-21 the ollama-cloud roster entry is a config override carrying all 23 cloud models; Ollama's native /api/show exposes capabilities + context length; every roster model has a declared window since 9c72bbb (pushed)"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

On 2026-10-21 the owner asked for every Ollama Cloud model to be available in
fleet. Written as a `config.json` provider override through
`fleet settings provider add` (the one writer): `ollama-cloud` now lists 23
models with `gemma4:31b` kept first so the provider's default model is
unchanged. `fleet settings provider remove ollama-cloud` reverts to the
compiled-in single-model default.

Discovery facts: the OpenAI-compatible `GET /v1/models` lists ids only, but
Ollama's native `POST https://ollama.com/api/show {"model": id}` returns
`capabilities` (`thinking`, `tools`, `vision`, and `decision` on `clef:27b`)
plus `model_info.<arch>.context_length` — real capability discovery, which
the other roster providers do not offer. Oddities seen: `minimax-m3` reports
0 params, `deepseek-v4.1-flash:1017` is a dated tag of the same model,
`clef:27b` has no `tools` (cannot drive the agent loop), and
`mistral-large-3:675b` has no `thinking`.

**Why:** the roster's `models` list feeds every picker (`.catalog.models` in
`/api/settings`), so this is what "more options for the configuration" means.
`CONTEXT_WINDOWS` in `src/route.rs` grew 11 → 36 rows in `9c72bbb` (pushed
with the router slice, ratified by the approved plan): the 23 ollama windows
from `/api/show`, liquid `lfm2.5-24b-a2b` at 131,072 (release coverage,
secondary source), deepseek 128K → 1M because the old `deepseek-chat` name
serves the V4.1 flash generation. The window test's named-absent list is
exactly `blend1/blend1`.

**How to apply:** when a provider publishes its windows, poll them (Ollama
does); otherwise research and declare. A model added to the ollama override
later still needs its window row, or it compacts at the 100k floor. Related:
[[fleet-compaction-budget-is-window-aware]],
[[no-form-asks-for-a-value-fleet-already-has]],
[[fleet-router-rules-everything-but-the-leader]].
