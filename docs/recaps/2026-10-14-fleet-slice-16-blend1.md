# Slice 16 — blend1: the grounded lane, and the Liquid provider (2026-10-14)

**Plan:** `docs/plans/2026-10-14-blend1-integration.md`.
**Feature:** `docs/features/blend1.md`. **Ledger:** KI-137..KI-162.

## What landed

The owner added a fifth lane to the specialist lanes — blend1 — and a
provider that complements sys1. This slice absorbs both.

- **The vendored lane** (`services/blend1/`): the core, the service,
  the LICENSE, and VENDOR.json recording the upstream commit. The
  pinned commit is `ab4ac55`, the tip of
  `feature/escalation-and-sys1-score` — **not merged to main**, which
  requires the owner's approval. The lane is Apache-2.0 (the plan's
  MIT was an unchecked assumption; KI-137).
- **The MCP adapter** (`service/mcp_server.py`, 566 lines, NEW —
  fleet's carry; the upstream ships a library plus a FastAPI service
  and no MCP server): five tools over stdio — `ask`, `doctor`,
  `search`, `refresh`, `seed` — typed named errors, no tracebacks, no
  key material on the wire, the sibling cores path-wired through the
  house `<SVC>_PYTHONPATH` mechanism.
- **The fifth LANES entry**: blend1, role "grounded", the union of the
  three siblings' package sets (blend1's own core is stdlib-only and
  composes them), the five path vars wired at spawn.
- **The probe's doctor card**: the lane's `doctor()` per-lane
  availability (route/semantic/tabular/generate/validate) rendered
  with status dots and named reasons — never a plain-text table — in
  both `fleet lanes` and the TUI's lanes modal, with wrapping at any
  width.
- **The Liquid provider** in the roster: id `liquid`, base
  `https://api.liquid.ai/openai/v1`, model `lfm2.5-24b-a2b`,
  `LIQUID_API_KEY` — appended at the END of the declared chain, whose
  order is the owner's standing directive; the owner's config decides
  its real position. d1's judgment already rides inside sys1 (no fleet
  change).
- **The escalation and the sys1-scored reprocess**, built upstream on
  the feature branch and vendored: an inconclusive or uncleared gate
  escalates to a larger configured model and ships flagged under
  `escalated_answer` (a new key beside `escalated`, whose meaning stays
  "a lane failed open"); d1 scores the generated answer on the same
  state shape the GLiDE critic reads, a wrong score reprocesses once,
  and an unavailable route lane names the absence — never a fabricated
  score. Both bounded; the gate never blocks.
- **The corpus tools and the scheduled refresh**: fourteen per-project
  cron jobs (`corpus-refresh-<project>`, daily 00:10–01:15 UTC,
  `auto_approve`, explicit absolute source globs), smoke-proven end to
  end — the lane joins, the refresh rebuilds, the store lands, a second
  fire reads `unchanged` in 0.6s. All fourteen project corpora are
  built (fleet 319, deepseek-harness 3279, dnd-vtt 2154, mahjong-vtt
  618, and ten more).
- **The eval corpus's four scenarios** — grounded-fact, escalation,
  reprocess, corpus-recall — grading fleet's side through scripted
  seams, no paid live calls.

## The derived use cases — problem-shape first

Seven shapes, each with the naive path that fails today, the pieces
that solve it (including the combinations), and its fleet surface:
**PS1** the cold-context question (corpus + grounded ask + search);
**PS2** the number that must be right (TabPFN computes, the reprocess
catches an altered number — combined with PS4); **PS3** the docs that
drifted (the scheduled refresh + the tower board's git facts —
combined with PS1); **PS4** the cheap answer that might be wrong (the
ladder: router gate → GLiDE retry → reprocess → escalation); **PS5**
the ecosystem's currency (a corpus per repo + the TinyFish web lane);
**PS6** the claim that must be checked (grounded ask + tabular +
reprocess + escalation — combined with PS2); **PS7** the context that
doesn't survive (fleet's `memories/` beside the corpus — separate
stores, separate jobs, documented not merged).

## Evidence

- Gate, parent-run on the merged tree: `cargo build --all-targets`
  exit 0, **zero warnings**; `cargo test` exit 0 — **517 unit + 6
  blend1 + 2 eval + 7 improve + 11 install + 90 integration, 2
  ignored**; `memories/` clean after the full run.
- The upstream's own suite: **66 passed, 0 failed** (43 before the two
  legs), verified by the parent through junitxml.
- The vendored tree: byte-exact against the upstream at ab4ac55
  (cmp-verified by the reviewer); the adapter handshakes for real
  (initialize, tools/list = five tools, the doctor report, the named
  refusals).
- Live through the real binary: `fleet lanes` renders five cards with
  blend1's doctor rows; the lanes modal's fifth card captured at
  120x40 and 80x24 with 20/20 deterministic checks; the wiring smoke's
  refresh round-trip.
- The reviews: spec **SPEC-COMPLIANT** (one must-fix, fixed); quality
  **REQUEST_CHANGES** (one critical — a false pre-build claim of the
  parent's — two importants, seven minors; all fixed in `b8762cc`).

## What the reviews caught

The quality review's critical was the parent's own overclaim: the plan
said fourteen corpora were pre-built when eleven existed — and the
three missing were exactly the ones that can never fit the bound.
Chasing it produced the session's sharpest finding (KI-149): the
corpus refresh's one-shot `sem1.embed` inherits http_llama's 60s
default and never raises it, so a corpus above ~500 chunks cannot
build through the lane at all — the kwarg exists, the module doesn't
use it. Underneath that (KI-150): the shared embedder had been killed
(launchctl status −9) mid-build; restarted with its own documented
kickstart, HTTP 200 in 0.38s. And the fix child caught what would
otherwise have read as a code regression (KI-162): four real-lane
tests fail under load while a corpus build holds the embedder's single
slot — the same failures at a pristine HEAD in a clean worktree.

## Not done

- The blend1-shaped subagent engine (the general template — the
  follow-on design), the FastAPI service surface in fleet, the boards'
  telemetry.
- KI-148/KI-149's bound fixes (a per-tool lane bound; the corpus
  module batching its embed) — logged open with their next actions.
- KI-161: the fourteen jobs are undeclared machine state — a fresh
  FLEET_HOME re-creates them from the plan's B13 record.
- The upstream carry (KI-143): the adapter belongs upstream, and the
  escalation's `escalated`-vs-`escalated_answer` semantics (KI-152)
  and the escalation/reprocess interaction (KI-158) are upstream
  questions, not forks.
