---
name: fleet-recall-measured-state
description: "The 2026-10-18 measurement of fleet's live recall, before AND after the fix plan — correct fact ranked #1 in 7 of 12 before, 8 of 12 after; the dominant remaining cause is the phrase-vocabulary gap; plus the n-gram (no) and knowledge-graph (staged yes) verdicts"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Measured 2026-10-18 by replaying the ranker over the live 24-fact corpus
(`memories/`) with twelve instructions whose answer exists, on a
keyword-only machine so the vector channel is 0. This is the evidence
base for the recall half of [[fleet-memory-cache-fix-plan-drafted]], and
it is now pinned **inside the gate** by
`the_measured_set_holds_the_floor_the_reweighting_bought`, which reads
the repo's own corpus through `CARGO_MANIFEST_DIR` — so the numbers are
the gate's own rather than a script's.

**Before.** The floor admits 71% of the corpus — 16.9 of 24 facts clear
`MIN_RECALL_SCORE` (0.20), and `ASSEMBLY_TOKEN_BUDGET` (2048 tokens ≈
25–30 facts) admits almost everything the floor admits, so the floor does
almost no noise control. The history channel is saturated: 16 of 24 facts
sit at or above `RECALL_CEILING` (8), median recall-count 56, so the
channel contributes a constant 0.250 to two-thirds of the corpus. The
standing term is 71% of the winning score. **Correct fact ranked #1 in 7
of 12**, top-3 in 10 of 12. Two of the five failures were exact ties
broken by the alphabet — `own-home-credentials-ruling` beat
`router-enforcement-ruling` at an identical 0.578 purely because `o`
precedes `r`.

**After (R1 + R2).** **8 of 12** at rank 1, **10 of 12** in the top
three. The single gain is `orchestrator-walks-the-hard-chain`, which the
alphabet had taken on an exact tie. That is the honest size of the win,
and the plan labels it as such: R1 and R2 are **correctness fixes** — a
saturated channel is a dead channel, and an alphabetical tie-break is an
arbitrary relevance signal — not the recall improvement.

**Morphology is not the problem** (a light stemmer recovers 6 of 99 query
tokens, 6%), **and neither is the standing term's size** — a fact with
full keyword coverage and a 0.5 trajectory match scores 0.481 against a
103-recall wordless fact's 0.400, so direction still wins. **The dominant
cause is that recorded `phrases` carry the resolution's vocabulary, not
the task's nouns:** `own-home-credentials-ruling` scores trajectory
**0.000** against "where do credentials resolve from" because its phrases
say `FLEET_HOME` and `.env`. In the twelve runs the word channels deliver
0.04–0.15, not the 0.60 they are capable of. **No reweighting reaches
this** — it is an authoring gap, and it is the next thing to fix.

**R1's side effect, recorded rather than hidden:** the log curve reads
four recalls as history 0.331 instead of 0.5, which narrowed the
documented direction-over-proximity margin from 0.095 to 0.053 and left
the direction-strong fact clearing `SPILLOVER_FLOOR` (0.50) by only
0.0028. The `the_two_signs…` test's ordering also flipped — a divergence
with a perfect trajectory match now leads a resolution with four recalls
— which is the ruling applied consistently, and safe because the sign is
on the line and the legend names it.

**N-gram channel: declined.** Wrong failure (6% of tokens), would move
every fact together so it reorders nothing where the ordering is already
wrong, would raise the 71% that already clear the floor, and costs a
seventh weight in a composite the owner has ruled on twice. If fuzzy
matching is ever wanted it should be a query-side typo correction against
the corpus vocabulary, not a channel.

**Knowledge graph: shipped, staged, and still empty.** The substrate
declares edges the retriever walked none of: `superseded-by` (0 populated
edges), `missing` (15 facts, all empty — so the third sign never fired),
shared `shape` (already read by the partition, deliberately not an edge).
R4b now requires `missing` on an `incomplete` fact. The graph does
nothing on the live corpus yet and that is expected — the edges arrive as
the corpus is used. See
[[an-edge-whose-source-is-retired-cannot-be-walked]] for the design
correction the `superseded-by` edge forced.

**Why:** the ranker's weighting has been re-litigated twice (the
direction-outranks-proximity ruling, then its own correction at KI-195),
so a reweighting is the wrong instrument and a measured failure mode is
the right one.

**How to apply:** do not re-derive this — re-run the twelve-instruction
set if the corpus or the weights change; the gate test is the harness.
Related: [[fleet-recall-bump-invalidates-the-embedding-cache]],
[[fleet-memory-cache-deep-dive-findings]],
[[verify-the-arithmetic-behind-a-measured-narrative]] (why the first
draft of this analysis was wrong).
