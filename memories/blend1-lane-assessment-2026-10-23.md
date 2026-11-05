---
name: blend1-lane-assessment-2026-10-23
description: "2026-10-23 assessment of the blend1 lane — ~3,082 lines of vendored Python (pin ab4ac55), fleet's retrieval substrate AND a deliberately portable intelligence service for OTHER harnesses (zcode etc.): three transports where the OpenAI-compatible /v1/chat/completions face is the distribution story; debts ranked — KI-297 non-convergence first, single-embedder coupling, the env/deployment shape being the real portability tax"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-23 assessment of the blend1 lane, from reading the vendored source:
~3,082 lines of Python per `services/blend1/VENDOR.json` (pin `ab4ac55`,
2026-10-14, upstream `~/Projects/blend1`). It turns docs-on-disk into an
embedding-indexed fact corpus (derive → hash-gated refresh → embed → search)
and exposes it to the model as one `ask` tool plus one `refresh` tool. It is
the retrieval substrate behind fleet's memory recall, the per-project corpus
snapshots, and composed-workload grounding — 42 commits touch it.

**Owner correction, same day — the assessment's frame was too fleet-centric.**
blend1 is meant to be used with OTHER harnesses (zcode etc.); the owner made
portability achievable on purpose. Reading the upstream confirms it holds:
three transports with different coupling budgets — (1) `import blend1` (full
five-lane ask(), heaviest coupling), (2) the FastAPI service on :8420 whose
`/v1/chat/completions` speaks the OpenAI envelope so ANY OpenAI-capable
harness (zcode, Codex, Cursor, Claude Desktop) uses it with zero integration,
(3) the MCP server (how fleet vendored it). The contract that makes adoption
free: **the truth rides beside the standard, never instead of it** — the naive
client reads `choices[0].message.content`, the curious one gets the non-
standard `blend1` field ({path, route, validation, escalated_answer,
cost_usd, latency_ms}). The HTTP posture is fail-open by design: never 500s,
an empty answer is a 200 with the reason in the `blend1` field, a gate
failure ships flagged — "a flag is information, an outage is an outage" —
because one harness's error handling must not become another's crash.

**The real portability tax is deployment shape, not transports**: the
dependency halo (sys1/sem1/sdm1 as path deps, the optional gen1 voice import,
keys across `~/.config/sdm1/env` and the process env, the local EmbeddingGemma
llama-server on 8901) plus a quiet contract — blend1 reads sdm1's env home
(`~/.config/sdm1/env`), discoverable only in env.py. `BLEND1_ENV_FILE` exists
as an override; what is missing is the env-resolution order written down as a
portability contract rather than folk knowledge. The chat/completions shim is
the distribution story to lean on: any of the 16 foreign configs the MCP scan
found could point at blend1 as a plain OpenAI provider today.

**Strengths:** the docstrings state their own limitations (the model-drift
blind spot is documented inside `refresh` itself); steady-state refresh is
file reads + hashes, embedding only on actual change, so the 14 scheduled
corpus refreshes are near-free when nothing changed; and it is the only local
model in the building (EmbeddingGemma-2 llama-server on 127.0.0.1:8901).

**Debts, ranked:**
1. **KI-297 non-convergence** — duplicate `##` headings make the change check
   compare a list-derived count against a set of (key, sha) pairs, so several
   of the 14 projects report changed/rebuilt forever, re-embedding identical
   text and silently dropping duplicates (`items` ≠ stored count). A
   data-structure fix (dedupe at derive, or set-vs-multiset honest check);
   highest trust-per-line payoff.
2. **Single-embedder coupling** — everything rides one local server whose
   health flaps (KI-296), and the documented model-drift blind spot means a
   silently changed embedding model is undetectable; a `model_version` field
   in the store manifest would close it.
3. **Boundary blur** — blend1's per-project corpora vs fleet's own
   `~/.fleet/memories` serve overlapping questions; the division (project
   docs → blend1, harness-native facts → fleet memory) deserves one written
   paragraph before the swarm work makes more agents write more facts.

Also noted: blend1 is the lane most punished by the cron architecture —
KI-256 (scheduled runs get no lane tools) and KI-294 (no skills) matter
*because* of it; closing those makes the corpus jobs sharper and cheaper.

**Owner follow-up, same day — the packaging ruling:** a seamless install
will come "eventually"; right now it is progressive elaboration, and the
deferral is deliberate because packaging freezes contracts. The intent
end-state: lanes configurable by users, blend1 "just a layer of coordinated
use of all the lanes." The full lineage (blend1 → dev-decisions →
agnostic-router-kit → zcode-router-kit → fleet) lives in
[[fleet-origin-lineage]].

**Why:** the owner asked for my thoughts on the lane; the answer's durable
half is the ranked debt list and the fact that the triad mapping and web leg
are already recorded elsewhere — this file is the lane's *current* state.

**How to apply:** before touching blend1 or the cron corpus jobs, check
KI-297's mechanism first; if adding embedding-side changes, add the manifest
model_version. When evaluating blend1 for use OUTSIDE fleet, judge the env
halo and degraded-mode config, not the transports — those already carry any
OpenAI client. Related: [[blend1-is-the-latency-triad]],
[[blend1-web-leg-and-corpus]], [[embeddings-cannot-label-a-session-project]],
[[fleet-judgment-beats-spawn-per-row]].
