---
name: super-agent-explorations-for-fleet
description: "From the offloop DeepSeek Super Agent article, three explorations rank for fleet — usage-in-calibration first, because the calibration stream verifiedly records no token accounting at all (RouteRow has provider/verdict/attempts/latency only; Usage carries prompt/completion/total and drops cache fields)."
type: design
date: 2026-11-02
basis: inferred
tier: durable
shape: [change, deps]
phrases: [super agent, router and worker sessions, delegation-first prompt, dsh-superagent, offloop article, scheduling experiment]
missing: [owner ruling on which exploration (if any) becomes a plan, provider-by-provider confirmation of which usage fields each of the five lanes actually returns (only DeepSeek's format confirmed so far)]
recall-count: 0
superseded-by: usage-and-anomalies-plan-exists
valid-until:
---

Source: https://offloop.org/learn/deepseek-super-agent-design (Nov 2026, on dsh-superagent / the DeepSeek Harness lineage fleet forked from). Its benchmark: router+workers P50 12.4min / 23.23M cache-read tokens vs single-session-with-subagents under a delegation-first prompt 13.8min / 10.82M vs neutral prompt 20.4min / 6.05M — a good delegation prompt closes ~90% of the latency gap at under half the token cost, so the architecture's residual value is ~1 P50 minute for a 2-4x cache multiple. Fleet's standing prompt already is delegation-first (structured brief, capability-scope, bounded delegation), so the article mostly VALIDATES the current single-orchestrator design; the owner's quota rulings (other plans first, DeepSeek in reserve) rule out the 4x cache multiple.

Ranked explorations (verified against source this session):
1. USAGE INTO THE CALIBRATION STREAM. Verified: RouteRow (src/calib.rs:64-97) records provider/how/workload/verdict/attempts/latency/input_chars — zero token fields; Usage (src/api.rs:491-500) carries prompt_tokens/completion_tokens/total_tokens only, so providers' cache_read/cache_creation tokens are parsed and silently dropped (serde ignores unknown fields). The article's entire economics argument (cache-read multiples) is therefore unmeasurable in fleet today, while spend is the owner's binding constraint (protected-tier quota rulings). Fix shape: optional serde-default cache fields on Usage, ride them into the calibration rows — same pattern as judged_from; old rows read as empty. Then "am I actually spending the other plans first" becomes a fleet calib query instead of a memory.
2. WATCHDOG AS CALIBRATION ANALYSIS, NOT A DAEMON. Verified: no stall/overrun/run-level anomaly detection exists in src (grep hits are all memory-trajectory vocabulary). max-turns bounds a delegation but nothing notices run-level patterns: consecutive 429 fallbacks walking the whole roster, a repeated identical tool-failure loop, lane+embeddings both down. Fleet already has the inputs (calibration stream, sys1 judge, KI ledger failure shapes). Native shape: a sys1 anomaly pass over recent stream rows surfaced post-run / next-turn (e.g. fleet calib anomalies) — the analysis half first, any runtime watch later, delivery-seam-style.
3. BRIEF COMPOSITION VIA RECALL. The article's ablation: removing direct delivery (events reaching the right worker) +37% tokens at equal time. Fleet's analogue is what a delegation brief carries vs what the child re-fetches; the structured brief already gets most of it. Small option: assemble a brief's context section from top recalled facts for the delegated task instead of the parent hand-picking.

Rejected, with reasons: event triggers/webhooks (violates nothing-enters-but-the-owner; the remote listener answers, never initiates); persistent worker sessions (violates bounded delegation, max-turns, KI-076 uncapped-occupancy); multi-repo scope (violates house rule 6, native Rust / no checkout dependency).
