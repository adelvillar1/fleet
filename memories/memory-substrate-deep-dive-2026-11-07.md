---
name: memory-substrate-deep-dive-2026-11-07
description: "project 2026-11-07 — the memories implementation deep dive landed as docs/architecture/deep-dive-2026-11-07-memory.md (13c6720); six findings led by the protected tier's 7× budget overrun; no plan drafted yet"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner asked 2026-11-07 for a deep dive on fleet's memories implementation; the full-fidelity survey landed at `docs/architecture/deep-dive-2026-11-07-memory.md` on `face-plugin-host` at `13c6720` (docs-only, no gate). Every mechanism verified sound (write-path refusals, KI-402/355/356 guards, non-blocking probe, digest+embedder-keyed index, fold-first consolidation, home-first root with `~/.fleet/memories` EMPTY — no shadow). Six findings, none acted on yet:

1. **Protected tier outgrew its budget** — 201 user/feedback facts ≈15,065 tokens rendered vs `ASSEMBLY_TOKEN_BUDGET = 2048`; the tier is contractually never dropped, so every turn carries ~7× budget before recall. Sized when the tier held 7 facts; the transplant grew it 29×. Remedy (cap/re-tier) is a plan's decision.
2. 89% of the 639-fact corpus has recall-count 0 (transplant-expected); ~7 veterans at 577–597 hold the top at the 1.0 history ceiling; `STALE_AFTER_DAYS=180` means the corpus grows monotonically for months.
3. Failure axis thin — only 10 facts carry failure shapes (fail-open 6) vs a ledger that closed 28 rows the prior session; 52 carry `resolved`, only 4 carry `missing`.
4. `state-sync` tagged on 154 facts (24%, 52 also `resolved`) — likely the transplant batches' default tag, diluting the partition.
5. The working tier is unused — zero facts carry `valid-until`, so compress has never had material.
6. Data hygiene: 3 facts carry a literal `—` type; episodic counts disagree by one (5 typed vs 4 tiered).

Also verified: the two-stage Matryoshka prefilter exists but is OFF in every constructor with no consumer (KI-252 gate hasn't flipped it); `CURATE_JUDGED_DECIDES = true` (the owner's award, rollback is one constant); the curate panel is glide/drex/jev fanned out in one sys1 call; the face writes `design-board-<slug>` facts confirmed-by through the ordinary writer; huddle room knowledge derives from the log's Memory records + approved remember results; `fleet hygiene` runs one consolidation pass.

Next step offered, not taken: a plan for the findings (tier cap + failure-shape distill pass carry the weight). A companion dive read Anthropic's agent-memory article against these findings — see [[memory-dreaming-comparison-2026-11-07]]; its area 2 is external confirmation of finding 1. See [[memory-transplant-plan-drafted]], [[house-methodology-implemented]].
