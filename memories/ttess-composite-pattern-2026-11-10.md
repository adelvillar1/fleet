---
name: ttess-composite-pattern-2026-11-10
description: "reference 2026-11-10 — the owner pointed at ~/Projects/TTESS (\"about 4 calls for the walkthroughs and the 45 min forms\") as a working composite pattern to review; the 45-min observation pipeline is 5 concurrent domain calls + aggregator + TypeSafe d1 predicate decisions + Fastino extraction + deterministic guards + provenance stamping — composes where domains are independent"
type: reference
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

The owner's own production composite pattern, offered 2026-11-10 during the
grounded-selection composite round ("maybe that's a pattern that you can
review"): `~/Projects/TTESS` (Next.js/TS, T-TESS teacher evaluation app).

**The 45-minute observation pipeline** (`lib/ai/tpess-pipeline.ts`): 5
concurrent domain calls (one per T-TESS domain, `MAX_DOMAIN_ATTEMPTS = 4`)
+ an aggregator call (retry 3) + **TypeSafe d1 predicate decisions**
(`lib/ai/typesafe/rate-indicators.ts` — narrow judgment heads) + **Fastino
extraction** (student-names, content-tagging) + **deterministic guards own
enforcement** (the name-leak scan is pure code; the LLM only shapes the
candidate set — confidence HIGH = LLM-extracted AND found, MEDIUM =
heuristic-only) + **provenance stamping** (`ratingSource: "model" |
"override"`) + `withModelFallback` provider chain.

**The walkthrough fill** (`lib/forms/fill-from-narrative.ts`): deterministic
candidate extraction → ONE strong model call with typed schema output →
deterministic validation → the leak guard → provenance. The fill-narrative
route streams 5s heartbeats (Railway closes idle connections at 60s).

**Why it composes and fleet's tool-selection doesn't:** TTESS's domains are
structurally INDEPENDENT chunks; fleet's tool candidates are mutually
competitive (choosing one = not-choosing others), so per-tool independent
judgments lose the ranking — measured: flat 51.3/76.5 beats every composite
(45.0/49.2/42.5 top-1). **The rule: decompose at independence joints;
deterministic scaffolding guards; decision models for narrow judgments;
provenance on every AI-produced value.** Fleet's C12/C17/C19 are this
pattern's instances. The owner's follow-up shape (weed-then-verify: flat
pass weeds to top-5, batched noul verifies survivors) found the one working
composite — the product of the comparative and independent signals, where
the noul pass improves top-3 on both vendors (same-vendor) — because it
cuts at the independence joint (verify each survivor) without asking
independent judgments to order competitive candidates.

**The owner's extension rounds (2026-11-10, all measured):** "you can mix
glide, gliner, drex, and d1 — each has its strengths" and "an lfm3 call in
the middle is also fine — the combinatorial possibilities are immense."
Measured answers: cross-VENDOR verification composes nothing (identical to
control — noul calibrations don't transfer), LFM3-in-the-middle degrades
(30.3/20.2%), the score fan-out collapses (27 heads → identical 5.0s, a
wire artifact), and the vendor's own canonical framing (described criteria
+ escape route) gains no top-1 and loses ~12pp top-3. The reproduced band
is same-vendor composed verification: 54.5–55.5% top-1 / 78–80% top-3.
**The one rule that survived every round: the answer must be IN the
information the question carries — selection-from-intent is
under-determined (10 shapes, 3 vendors), while independent-property
questions (destructive, verification, validity) were sharp every time.**
**The closing proof — the verifier measurement (2026-11-10 evening):**
on the mechanically labeled corrupted-call corpus, glide separates
originals from corrupted calls at **AUC 0.983 and rejects 60/60
corruptions** (drex 0.883/96.7%) — the TTESS-shaped independent-property
question is exactly as sharp as the rule predicts, while every
selection-shaped question had failed. The asymmetry (originals accepted
only 41.7–45.0% — conservative acceptors) sets the composition design:
verification composes with the model's own signal and never gates alone.
Related: [[grounded-selection-expanded-tier123-2026-11-10]].
