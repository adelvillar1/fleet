---
name: strategist-deep-dive-2026-11-10
description: "project 2026-11-10 — Princeton/Meta's Strategist (arxiv 2611.03618) read against fleet's self-improvement loop; landed as docs/architecture/deep-dive-2026-11-10-strategist.md (b3b4fea, docs-only); four improvement areas led by the lesson store being filtered not searched, with the no-library ablation (-13.3 pts) as the price"
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

Second deep dive of 2026-11-10, same pattern as [[ernie-evolve-deep-dive-
2026-11-10]] (companion doc; the user said "use the same pattern with this
article"). *Strategist: Evolving Reasoning with Agents* (Lu, Xu, Hua, Liu —
Princeton/Meta, arXiv 2611.03618v1): an AI system is half model, half
inference strategy, and the strategy half must become cumulative — executable
**blueprints** (one interface `execute(llm, task) -> str`, composable under
four protocols), a shared **library** carrying each blueprint's measurement
card + experience notes, and a four-step loop (Retrieve with a relevance
query AND a deliberate-drift inspiration query → Propose K=5 with forced
new/composed/reuse diversity → Evaluate over four splits with a 2%-rule →
Accumulate blueprints-not-prompts, AST-dedup, warnings fed forward).
Results: 24/30 wins, 74.7% vs 74.0% at −39% cost; library-alone within
2pts on all 10 held-out; **the ablation that prices it: iterate without the
library and you lose 13.3 pts — "without inspirations, it goes on to a
narrow repetition."**

**Landed:** `docs/architecture/deep-dive-2026-11-10-strategist.md`
(`b3b4fea`, docs-only → gate skipped; tower t-20261110-123844-4b0a).
Every fleet claim read at its file:line this session.

**Four improvement areas (findings, not decisions):**
1. **The lesson store is filtered, not searched** — `lessons_for`
   (improve.rs:745) is target-kind + five newest; lessons live in
   `<home>/lessons/` (improve.rs:593) outside the recall corpus
   (memory.rs:1-12 is `memories/` only). Their ablation prices this.
   Candidate: query-time read over lessons/capabilities through the
   EXISTING hybrid engine (lexical.rs:18 BM25, fusion.rs:12 RRF-60,
   hybrid.rs:224), indexed on their trio, **two queries: relevance +
   deliberate-drift inspiration**, dropped into the filing confirmation
   where the five lessons already appear (improve.rs:1141). Boundary:
   read-time only, never write lessons into the git-tracked fact
   substrate. Ship MEASURED (hits actually used) — fleet files rarely.
2. **Cards carry confidence but no numbers, and are never revised** —
   `Lesson::from_proposal`'s `measured` (improve.rs:630) means "corpus
   was green at apply," a boolean. ERNIE area 5 already queued the
   settlement readback; Strategist adds the DESTINATION SHAPE: write
   settlement INTO the card as measured numbers + when-it-works, and
   make the card part of the retrieval index. Borrow their honesty
   rule: quote the untouched number, keep selection signal separate
   from teaching signal (their hidden pick flatters +4.3 vs +0.7 test).
3. **Failures never reach the next attempt — two articles now.**
   Their AST warnings feed the next iteration's prompt; fleet's lessons
   write ONLY on apply (finish's Applied arm, improve.rs:1835) and
   refusals ride only their own file (:1875). Same fix as ERNIE area
   3, same seam: filing confirmation lists recent refusals beside the
   lessons.
4. **Their search-side fields are enforced; our analysis-first is
   prose** — `search_queries/read_cards/experience` are schema-checked;
   fleet's analyze-FIRST is a tool description (tools.rs:1747)
   nothing verifies. Cheap form: record whether an `analyze` row exists
   for a nearby window and say "no analysis on record" on the filing
   reply — named, not refused.

**Where fleet is ahead:** deterministic scorer (their splits exist
because dev sets are noisy), the twin/sham placebo control (improve.rs:
2862 — no per-candidate equivalent there), provenance on remembered
things (basis/confirmed-by vs free-text notes), human-gated landing
(their `s*` ships itself; our modes never will), and the retrieval
engine itself.

**Non-adoptions priced:** the K=5×N=10 slate search (needs a fixed
scorer; quotas would file unevidenced proposals), the blueprint graph
(the compiled harness IS fleet's strategy layer), per-step tier
routing (different economics), the four-way split machinery (borrow
signal separation when settlement lands, not the machinery).

**FOLDED IN (the owner's "can we fold your recommendations into the
previous plan?"):** areas 1, 3, 4 → **C10–C12** of
[[improvement-modes-inbox-plan-drafted-2026-11-10]] (content-query
lessons with the cross-target inspiration hit; refusals listed at
filing — same seam ERNIE area 3 wanted, so one fold absorbed both
dives; "no analysis on record" within `ANALYZE_WINDOW_DAYS`). The
filing path this plan already edits is what made the fold clean — one
pass over `file()` covers mode-authorization and all three reads.
**Area 2 (card numbers) stayed OUT** by my call, stated in the plan's
Out of scope: it waits on ERNIE area 5's settlement readback, which no
plan builds; C10 only gives those cards the retrieval they'd be ranked
by.

**Open:** the remaining area-2 question (cards wait on settlement or
ship apply-time numbers) now sits with the settlement plan; whether
fleet's proposal volume justifies the inspiration query (the plan
carries this as a risk — the reply carries the hits so the session log
measures use, and one line retires the drift query if unused); how the
attempt seam interacts with lesson accumulation if autonomous mode
files fast.
Related: [[ernie-evolve-deep-dive-2026-11-10]],
[[improvement-modes-inbox-plan-drafted-2026-11-10]],
[[self-improvement-deep-dive-2026-11-07]].
