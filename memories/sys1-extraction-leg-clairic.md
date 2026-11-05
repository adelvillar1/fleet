---
name: sys1-extraction-leg-clairic
description: "Owner direction 2026-10-19 — Cloudflare's Clairic (System One EXTRACTION class) gives sys1 (decision-only today) its extract half; four integration points mapped, not yet approved for build"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner shared the Clairic announcement 2026-10-19 and asked how it improves sys1. the PLAN is drafted in the README spec format and committed as **d23f0f3**: `docs/plans/2026-10-19-sys1-extraction-leg.md`, `status: draft` — approval (draft→active) is the owner's act; the graph generates only then. **2026-10-25 UPDATE — the premise is challenged.** The owner asked whether GLiNER 2.5 (already wired to sys1 via Fastino) doesn't already do this. My assessment: substantially yes for the extract verb; Clairic's real deltas (estimate/forecast) are the kinds our calibration law refuses, and p_null is answerable structurally from GLiNER's spans. The leg landed (E1/E2, see [[extraction-leg-e0-blocked-e1-e2-landed]]) but is gated and unused; the recommendation on record is to re-point the extract verb at Fastino's GLiNER 2.5 schema endpoint. Awaiting the owner's ruling before building either direction.

**sys1 today** (verified in code): decision-class only — `SYS1_DECISION_KEYS = ["FASTINO_API_KEY", "TYPESAFE_API_KEY"]` (route.rs:1392), `JUDGE_LANE = "sys1"`, `sys1_classify` answers the workload+execution judgment, availability cached per KI-044. Lane map: glide→FASTINO, jev→TYPESAFE.

**Clairic** (Cloudflare, Oct 2026): System One EXTRACTION class — extract typed values with spans+confidence, compute one-pass values, estimate distributions (median + 80% interval), forecast step series. No output tokens, sub-second, `p_null` instead of hallucinated guesses. Served on Workers AI (`@cf/cloudflare/clairic`); open weights promised → self-hostable later beside the local embed server.

**The plan's increments** (E0–E6; sys1 first, fleet inherits, deck demoted to one consumer among four):
- **E0** live wire probe before any typed model is written — reconcile documented vs wire schema, measure per-verb latency so E1's timeouts are born measured (KI-249 discipline at birth).
- **E1** the leg: `src/extract.rs` on the HttpEmbedder pattern (one ureq call, zero new crates, deliberately NOT an MCP lane process), four verbs (extract/compute/estimate/forecast), typed response (value, span, confidence, p_null, interval), credentials `CLOUDFLARE_ACCOUNT_ID` + `WORKERS_AI_API_TOKEN` through the vault overlay, `fleet doctor` probes it, `sys1_extracts()` is its OWN once-per-process gate (never a member of SYS1_DECISION_KEYS — the two availabilities are orthogonal), one retry for the stale keep-alive (KI-243's mechanism), and base URL + model id as config so the open-weights release self-hosts beside the embed server with no code change (own-home, designed now, deployed later).
- **E2** the `extract` tool — the general agent surface, nothing deck-shaped; declared in its spec (an undeclared mode is invisible).
- **E3** calibration inherits — typed outcome evidence feeds the judge, AND the same extractor over a day of logs produces KI-249's duration distribution; calibration accepts only extract/compute at the type level (a forecast can never be a gate input).
- **E4** deck inherits — typed values with rendered provenance, estimates as visible 80% uncertainty bands.
- **E5** card bus inherits — data-shape triggers (numbers→chart card, dates→timeline card, below-floor→verify card), onto the existing bus with fixes pre-baked.
- **E6** web lane inherits — extract-before-read, opt-in per call, raw path byte-for-byte unchanged.

**Laws written into the criteria:** confidence and p_null are gate inputs from day one (doubt never passes silently); Rust does the arithmetic (the model extracts single values only; the deterministic checks don't move); forecasts are illustrative only.

**Caveats (blog's own, fleet-mapped):** Rust does multi-step arithmetic (Clairic extracts/computes single values only); forecasts are illustrative (deck bands, never calibration numbers); low confidence routes to a gate or card, never silently passes.

**Architecture in fleet's shape:** a `clairic` lane (Workers AI REST = one ureq HTTPS call, zero new crates), `WORKERS_AI_API_TOKEN` in ~/.fleet/.env under the vault overlay (KI-250 pattern); an `extract` tool for the agent plus internal calls from calib and the deck pipeline; it does NOT join SYS1_DECISION_KEYS — extraction availability is its own lane gate. First move when approved: a live probe of the wire schema (same as the deck-suggest live embed probe). Related: [[fleet-deck-visual-pipeline]], [[fleet-recall-measured-state]].
