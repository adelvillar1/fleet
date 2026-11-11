---
name: memory-dreaming-comparison-2026-11-07
description: "project 2026-11-07 — the Anthropic \"components of agent memory\" article read against fleet; the dreaming comparison landed as docs/architecture/deep-dive-2026-11-07-memory-dreaming.md (6c0b9a3); seven improvement areas, no plan drafted"
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

The owner asked 2026-11-07 (before engaging the [[memory-substrate-deep-dive-2026-11-07]] findings) for a deep dive on Anthropic's "The components of agent memory" (published 2026-11-06, fetched at full fidelity). Landed at `docs/architecture/deep-dive-2026-11-07-memory-dreaming.md` on `face-plugin-host` at `6c0b9a3` (docs-only, no gate). Core verdict: **dreaming (model-driven reflection over transcripts, offline) is the half fleet has not built** — fleet's consolidation is deterministic janitorial work and `memory.rs` never reads a transcript. Every prerequisite exists: hard rule 5 makes transcripts replayable (the article's ground truth), slice 12 pre-authorizes superseding inferred facts on justification, `spawn_consolidator` is the scheduler seam, `improve` proposals are the gated landing.

Seven improvement areas (article evidence → fleet state):
1. **The dreaming pass** — validate inferred facts against their transcripts, propose corrections via supersede, populate the graph edges, distill cross-session patterns into gated proposals.
2. **Injected layer must stay small** — the article's promotion section (Claude Code: 200 lines / 25 KB index, adherence degrades, wrong notes ride every session) is external confirmation of the substrate dive's protected-tier finding (201 facts ≈15k tokens vs 2048 budget). The cap, not the cycle, is where this lands (the cycle names confirmed-by untouched).
3. **Evidence pointer** — `Remember` carries basis but no session/source field; one optional `source` frontmatter field turns dream-time validation from archive search into reading the named session. The cheap unblock for area 1; also serves the phrases-gaming limitation.
4. **Salience bar** — fleet's 89% never-recalled census is the article's Sonnet 3.7 trajectory (write everything → maintain memory instead of working); memory is for what the environment doesn't say back. Note the article's counter-finding: "record only failures" LOST the boost — a salience bar, not a narrower taxonomy.
5. **Graph as wiki seam** — 4 missing + 42 superseded-by edges across 639 facts; hub-and-link subject facts maintained by dreaming; the Borges bound is fleet's own (512-char descriptions abstract, not transcribe).
6. **Cadence by access** — the calib stream's `recall_hit` rows are the access log; the article's open question already has fleet's answer.
7. **Close the learning loop** — `fleet judge`/`delegate`/eval end in calib rows, not corpus corrections; the seam is improve proposals + supersede.

Two owner-ruling tensions named, not resolved: (a) "no model-written memory prose" vs dreaming output — the house shape is corrections through supersede + new facts through improve proposals, but auto-landing on inferred facts is the owner's call; (b) fleet's prescriptive write path vs the article's freedom-to-judge-salience finding — structure is load-bearing downstream, so watch write rate and recall coverage instead of removing the schema; (c, added with the fold) the basis vocabulary is the permission model's input, so the third `validated` value is a schema + ruling change, not a stamp (Basis::parse refuses it today).

**AMENDED at `fa2aaf8` (2026-11-07, owner steer):** the validation movement folded into areas 1/3 — sys1 judges fact+transcript (never validates in the abstract; the Mt. Moon note was wrong from the start), the third basis value is `validated` (never `judged` — taken by the route-judgment plan vocabulary; see [[sys1-validates-inferences-design-2026-11-07]]), fail-safe (lane fails → stays inferred), recall_hit-priority queue, measured-set gate before auto-upgrade.

Where the article VALIDATES fleet: basis = "how it knew the rule was true"; failure shapes/three signs = trap warnings; supersede = corrections; recall-count = replay marking; the flock + taken-name refusal = the concurrent-overwrite hazard; bounded fact sizes = the Borges bound. See [[memory-transplant-plan-drafted]], [[house-methodology-implemented]].
