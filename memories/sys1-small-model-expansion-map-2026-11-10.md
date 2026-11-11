---
name: sys1-small-model-expansion-map-2026-11-10
description: "project 2026-11-10 — exploration ask \"other surfaces for sys1 + Liquid small models, forget house rules\": a 23-item map in three tiers (small-model text replacement, heuristic→head, untouched Liquid capabilities); compaction is the biggest spend; the pattern is one seam — a head-client call in front of a string rule"
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

Owner ask 2026-11-10, before activating the grounded-selection plan: *"are
there any other surfaces that would benefit from expanding sys1 use and use of
smaller models from liquidai? forget about house rules for a moment, this is
exploration to get better at doing things."* A subagent mapped the crate for
heuristic decision points, big-model text regeneration, ranking seams, and
what already rides sys1. Three tiers, 23 items — **not a plan, an exploration
ledger**; the owner has not chosen any of it yet.

**Tier 1 — small generative model (LFM3-1.7B) replaces flagship text
generation.** Biggest first:
1. **Compaction summarization** (`agent.rs:4200-4280`) folds through "the
   same routed path every other request takes, no separate model" — the
   single largest cumulative flagship spend on the list; pure summarization
   is exactly the 1.7B's job.
2. Conversation/session titles — deterministic ladder today
   (`gui/bridge.rs:8089`), no model at all; visible quality gap.
3. `session::summarize` archive entries (`session.rs:943-1102`) — heuristic
   composition; small model writes recall-index entries that read like the
   session.
4. tool_advisor advice prose (`tool_advisor.rs:5,308-417`) — explicitly
   "no model", canned strings from counts; counts stay deterministic, the
   ADVICE text is the small-model job.
5. Improve proposal prose (`improve.rs:1150-1195`) — flagship fills
   justification/acceptance/effect; change block stays validated, prose is
   filler.
6. Skill self-authoring drafter (`skills.rs:1110-1130`) — pure record rules;
   small model drafts the skill BODY from the recorded failure.
7. Anomaly narration + doctor diagnosis recap (`anomalies.rs:11,388`;
   `doctor.rs:1024-1058`) — deterministic templates + up to two lane asks.
8. Delegate briefs (`work_brief.rs`) — small model drafts the child's brief
   from the goal.

**Tier 2 — heuristic → head (sys1 choice/score replaces a string rule):**
9. **Destructive-command detection** (`approvals.rs:149,340`) — the
   `DESTRUCTIVE` substring list has a second-stage `judge_destructive` head
   (`:1050-1093`) as *fallback*; **invert it** — head first, substring as
   fast path (today most commands never reach the head).
10. **Memory shape classification** (`memory.rs:1961,2177,2262`) — three
    hand-maintained keyword tables the demand writer *patches by
    string-anchoring on const text* (`demand.rs:566-605`); the most brittle
    seam in the crate, textbook classification.
11. Mention-kind rule — first-3-words match "review|critique|audit"
    (`session.rs:151`).
12. Silent-stop ack word list ("done","ok") gating report-repair
    (`agent.rs:106`); a yes/no head on "did this turn actually report?"
    beats an ack list (false negatives cost a wasted repair turn).
13. Skill merge/duplicate curation — token-Jaccard 0.6/0.8
    (`skills.rs:77,1802`) + meaning floor (`:706`); "do these overlap?" is
    a judgment, not a ratio.
14. **Recall rerank** — six hand-tuned channel weights + floors
    (`memory.rs:1528-1660`): keep retrieval deterministic, add a score head
    over the top-N; the weights stop needing hand-fitting.
15. Lesson coverage gate — stopword-token overlap ≥0.8 (`lesson.rs:616`).
16. Cron catch-up — pure schedule rule (`cron.rs:2185-2226`); a score head
    "is this missed run still worth firing?" prevents stale catch-ups.
17. Inbox triage/priority — five deterministic rows (`gui/bridge.rs:4106-4110`).
18. Boards handoff `next_score ≥ 0.6` (`boards.rs:2792`).
19. MCP schema readability keyword lists (`mcp.rs:983,996`) — GLiNER-style
    extraction (already on the keychain) does this properly.

**Tier 3 — Liquid capabilities fleet hasn't touched:** 20. LFM3-VL-2B
rendered-output/perception QC (sys1's `vision_qc` task already liquid-only,
`tasks.py:461-463`); 21. Liquid nano family (encoders/embeddings/ColBERT,
230M–2.6B) as a SECOND embedder for recall — sem1 vs Liquid disagreement
flags a fact for dream/validate (dual-embedder consensus, new pattern);
22. LFM3-1.7B as the universal form-filler (remember JSON, improve change
blocks, skill frontmatter — schema in, valid JSON out); 23. local d1/GLiDE
weights (MLX/llama ports exist) — removes the network hop from every head
call across all 19 Tier-1/2 sites (~50ms vs a lane round-trip).

**Already routed through sys1 (the map's baseline, do not re-propose):**
route select + judge beats, delegate probe, extraction leg, desk coverage,
dream/validate verdicts, curation best-of-3 (glide/drex/jev), work brief,
perception, anomaly triage, explain narration, the tool-router capability
head, and `judge_destructive` as fallback.

**The compounding observation (owner's frame — they asked for it):** Tier 1
and 2 compose into one shape — *the flagship plans and authors, heads grade
and classify, small models fill and narrate* — and every replacement feeds
the same graded stream the grounded-selection plan's C9 builds, so expansions
here are the demand map's feedstock, not a separate program. **Leverage
ranking I gave:** compaction→LFM3, destructive-detection inversion,
shape-keyword tables→head, recall rerank, titles.

**RESOLVED 2026-11-10 — folded into the plan, then EXECUTED:** the owner
picked *"let's include tier 1 and 2 into the plan"* + *"tier 3 if it's a
hosted api model is fine"* + *"set aside local models … this environment
doesn't much in terms of memory"* — Tier 1 → plan Phases 4–5, hosted Tier 3 →
Phase 6, item 23 (local weights) → Out of scope. **Execution outcomes:** item
1 (compaction), 2 (titles), 3 (archive summaries), 6 (skill drafter), 4
(tools-advise card) LANDED through blend1's new `generate` tool; 9
(destructive inversion) LANDED after its measurement passed (head 1/1+76/77
vs list 0/1+69/77); 20 (vision_qc over MCP with images) LANDED; 22
(form-filler) LANDED; 21 (second embedder) KILLED — Liquid serves no hosted
embeddings; 10 (shape tables) KILLED at measurement (head 62% vs the corpus's
own labels); 5/7/8 dispositioned on design authority (mid-run authorship
stays with the routed model; doctor's recap stays per its own plan's AC2;
anomalies already rides the explain lane); 11–18 stay open behind hand-
labeled fixtures. See [[grounded-selection-expanded-tier123-2026-11-10]].
This file remains the full item map with file:line.

**How to apply:** the owner framed this as pure exploration ("forget house
rules"); present options mapped to the estate, never audit them against
doctrine (see [[tool-calling-reliability-glide-lfm3-2026-11-10]]'s correction
#3). Related:
[[read-the-services-own-config-first]] (the subagent map is file:line
verified, but new claims still get read at the source).
