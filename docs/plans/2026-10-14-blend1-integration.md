# Slice 16 — blend1: the grounded lane, and the Liquid provider (2026-10-14)

status: landed-pending-review

## Goal

The owner added a fifth lane to the specialist lanes — `blend1`
(`~/Projects/blend1`, `main`, clean, v0.1.0, MIT) — and a provider that
complements the sys1 lane. blend1 is the blended lane of the sys1 house
pattern: one `ask()` that runs the d1 router (sys1), the local
EmbeddingGemma-2 semantic lane (sem1), and the Prior Labs TabPFN
tabular lane (sdm1) in parallel, then synthesizes with LFM2.5-24B-A2B
(Liquid's own API) only when the router clears `ROUTE_FLOOR` (0.70) —
a cheap model whose reasoning is grounded on facts, with a validation
gate (GLiDE → d1) and critic chains behind it. Its core is stdlib-only
with the three sibling lanes as path dependencies: it composes them, it
does not reimplement them.

This slice absorbs both into fleet: blend1 as the fifth vendored lane,
callable in-run, and Liquid as a declared provider candidate.

## The lane's full functionality (verified against the upstream, 2026-10-14)

Read from `~/Projects/blend1` at commit `d9c703c`: `ask()` runs the route
lane SOLO first (the front door — d1 via sys1), then the fact pool
(semantic + tabular + web) concurrently; `needs_generation` (noul ≥
`ROUTE_FLOOR` 0.70) gates the paid synthesis leg; the web lane fires on
`needs_live_facts` (fails CLOSED — an unfired free lane costs latency
only); the validation gate is GLiDE's rating with **one bounded retry**
(the verdict rides back as feedback; worst case ≈ 2x generate cost,
failures only) and an inconclusive outcome (`passed None`, no critic
answered) **ships ungated — it does not retry**; the retry that fails
ships the ORIGINAL answer, flagged — the gate never blocks; a down critic
ships ungated. The corpus (`src/blend1/corpus.py`) is per-project
semantic memory on sem1's VectorStore under
`~/.local/share/blend1/vectors/<project>/<slug>/`: `seed` (idempotent by
key), `search` (cosine, best first), `refresh` (docs-derived,
hash-checked staleness — the steady state is file reads, no embedding),
`migrate_legacy`; `CLAUDE.local.md` is never in a corpus (other processes
read these stores).

**Two capabilities the owner named that are NOT in the code yet** (no
branch, no stash, clean tree, no references — verified): an escalation to
a larger model when the result is inconclusive, and a reprocess when
sys1 scores the LFM output wrong. They are built upstream first (B10,
B11), then vendored — fleet does not fork lane cores.

## Acceptance criteria

- **B1 — the vendored lane.** `services/blend1/` carries the core
  (`src/blend1/`), the service (`service/mcp_server.py`, `main.py`,
  `pyproject.toml`), the LICENSE, and `VENDOR.json` recording the
  upstream commit (matching the sibling lanes' vendored shape; no
  `.venv`, no `__pycache__`). The lane layout check
  (`service/mcp_server.py` + `src/`) passes.
- **B2 — the adapter.** `service/mcp_server.py` speaks MCP 2024-11-05
  over stdio and exposes two tools: `ask` (`prompt` required; `table`,
  `target`, `task_type`, `retrieve` optional) and `doctor`. The house
  adapter shape: no tracebacks (typed, named errors), the env scrubbing
  the other lanes carry (the lane's own keys pass; `DEEPSEEK_API_KEY`
  does not), the sibling cores path-wired through the
  `<SVC>_PYTHONPATH` mechanism the other adapters already use.
- **B3 — the lane boots through fleet's own client.** initialize +
  tools/list + a call, exactly as the other four lanes; the venv
  bootstrap is the fifth `LANES` entry (blend1's package set — the
  three siblings' sets, since its core is stdlib-only — with the cores
  themselves path-wired); `fleet install --lanes` builds it and
  `fleet lanes` probes it.
- **B4 — the probe card.** `fleet lanes` renders blend1's card at the
  house visual bar: the per-lane availability from `doctor()` (route,
  semantic, tabular, generate, validate — each with its reason when
  unavailable), never a plain-text table.
- **B5 — in-run.** The agent calls the lane's `ask` inside a run; the
  grounded path is exercised end to end against a scripted seam (the
  house lane-test shape); a lane that fails open records its reason in
  the result and `escalated` is surfaced — never silently degraded.
- **B6 — the provider.** `route.rs`'s registry gains Liquid as a
  declared candidate (`id "liquid"`, base
  `https://api.liquid.ai/openai/v1`, model `lfm2.5-24b-a2b`,
  `key_env "LIQUID_API_KEY"`, key required) — available to the owner's
  config (pin or chain) **without reordering the declared default
  chain**, whose order is the owner's standing directive. The docs name
  that d1's judgment path already rides inside sys1 (no fleet change).
- **B7 — the eval corpus.** A blend1 scenario grades the grounded path
  (a tabular fact reaches the final answer unaltered — "LLMs do not do
  math; TabPFN does"), and the improve gate's regression guard covers
  it.
- **B8 — docs and UX.** `docs/features/blend1.md`; the lanes modal's
  fifth card; the README's lane list; English-only; no secrets; every
  error or warning encountered lands in `docs/known-issues.md`.
- **B9 — the gate.** `cargo build --all-targets` warning-free and the
  full test suite green, nothing deleted (KI-007's rustfmt/clippy
  absence disclosed).
- **B10 — the escalation leg (built upstream, then vendored).** When the
  validation is inconclusive (`passed None` — no critic answered) or the
  bounded retry does not clear the gate, the answer escalates to a
  larger configured model with the same givens (prompt + computed facts
  + semantic neighbors + web excerpts, the untrusted-source framing
  intact). The escalated answer ships **flagged** — a NEW result key
  beside `escalated` (whose meaning stays "a lane failed open"; the
  result-key contract is additive). The gate still never blocks: if the
  escalation itself fails, the original answer ships with the failure
  named. Bounded like the retry (one escalation attempt; the worst case
  is named in the doc).
- **B11 — the sys1-scored reprocess (built upstream, then vendored).**
  sys1 (d1) scores the generated answer's correctness against the
  givens; a wrong score reprocesses (re-synthesis with the score's
  feedback, bounded — the existing retry's discipline applied to the
  sys1 verdict). The outcome rides in the result: the score, whether a
  reprocess fired, and which answer shipped. The GLiDE gate stays; the
  sys1 score is the correctness judgment beside it.
- **B12 — the corpus tools on the adapter.** Beside `ask` and `doctor`:
  `search` (semantic search over a project's corpus — the agent recalls
  project facts in-run), `refresh` (docs-derived re-derivation,
  hash-gated — the steady state is file reads), and `seed` (idempotent
  by key — the corpus's write path, the TinyFish-sourced facts' door).
  Project resolution follows the lane's own rule (param >
  `BLEND1_PROJECT` > cwd repo name > legacy).
- **B13 — the scheduled corpus refresh, wired.** A cron job per project
  runs the refresh on its schedule (`auto_approve: true` — it is
  declared, idempotent, and hash-gated); the job's shape, the missed-
  window behavior, and the projects list are derived, created, and
  smoke-proven. Fleet's `memories/` and blend1's corpus stay separate
  stores with separate jobs (the agent's durable facts vs the project's
  facts) — the division is documented, not merged.
- **B14 — the eval scenarios.** Deterministic scenarios (scripted seams,
  no paid live calls) for: the grounded path (a tabular fact reaches the
  answer unaltered), the escalation (an inconclusive validation
  escalates and the answer is flagged), the reprocess (a sys1-wrong
  score reprocesses and the corrected answer ships), and the corpus
  recall (a seeded fact is retrieved and quoted unaltered).
- **B15 — the derived use cases, documented.**
  `docs/features/blend1.md` carries the derived use cases with their
  wiring: (1) the grounded ask in-run — the agent calls the lane when
  reasoning must stand on facts; (2) project-fact recall — the agent
  searches the project's corpus mid-run; (3) the scheduled corpus
  refresh — the project's facts stay current without a manual step;
  (4) the escalation — an inconclusive answer costs one larger call, not
  a silently weaker one; (5) the reprocess — a wrong answer is corrected
  before it ships, not after. Each names its fleet surface (the tool,
  the cron job, the eval scenario, the card).

## Design decisions

1. **blend1 is a lane, not a chain member.** Its value is the grounded
   `ask()` — a tool the agent calls when reasoning must stand on
   facts — not a chat model in the routing chain. The chain's order is
   the owner's explicit directive and this slice does not touch it.
2. **The adapter is new, in the vendored tree.** The other four lanes
   each ship a `service/mcp_server.py`; blend1 has none (it is a
   library plus an optional FastAPI service). The MCP path is fleet's
   lane contract, so the adapter is written here, in the vendored copy,
   and the upstream carry is logged.
3. **One venv, path-wired cores.** blend1 imports all three sibling
   cores as libraries, so its venv carries the union of their package
   sets and the cores resolve through the existing
   `<SVC>_PYTHONPATH` mechanism — no second copy of torch beyond what
   the lanes already carry independently.
4. **Liquid enters the registry, the owner's config enters the chain.**
   The code declares what the provider is; the chain's order stays in
   `FLEET_HOME/config.json`, where the owner's directive lives.

## Out of scope

- The blend1-shaped subagent engine (route gate → parallel lanes →
  synthesis → validation → escalation as the general template for
  fleet's own subagents) — the follow-on design, named not built.
- The FastAPI service surface (port 8420) — the MCP adapter is fleet's
  path; the service stays the other languages' door.
- blend1's telemetry in the boards (its redacted JSONL log beside the
  calibration stream) — a small follow-on once the lane is live.

## Assumptions (listed, not asked)

- The declared default chain keeps its order (router kit → z.ai → mimo
  → stepfun → token plan → deepseek); Liquid is declared and available
  to the owner's config, and moving it into the chain is a one-line
  config change whenever the owner wants it.
- The vendored tree matches the sibling lanes' shape; the upstream's
  own tests stay upstream (fleet's suite covers the lane).
