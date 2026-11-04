---
name: memory-transplant-plan-drafted
description: "The plan to transplant ZCode's 440 fleet memories into Fleet's substrate is DRAFTED and gated 2026-11-04 — procedure over fleet's own remember writer, curation by census"
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

The owner asked (2026-11-04) to transplant ZCode's session memory about
fleet into Fleet's own substrate ("fleet as smart as zcode on fleet
itself"). The numbers: Fleet's corpus 164 facts, ZCode's 440, exactly 2
slugs overlap — nearly pure gain. Both formats are one-markdown-file-
per-fact siblings, so the plan
(`docs/plans/2026-11-04-memory-transplant.md`, status draft, 11
criteria C1–C11, plan-gate PASS 11/11, arch-gate PASS) makes the
transplant a PROCEDURE over `fleet remember` (the CLI speaks the
writer's whole shape: --name/--description/--type/--shape/--basis/
--body/--phrases), never a converter — no new frontmatter dialect, no
binary mechanism.

Curation is the work, encoded as a committed census
(`scripts/memory-transplant-census.py`, python3 -I, deterministic):
tiering (transplant = fleet's domain; skip = this harness's own
behavior — the named class), superseded chains trim to the live tip,
dedup vs the 164 by description overlap, and a SCREENING pass for the
public repo (machine paths, key names, sibling-repo internals deeper
than the corpus already carries — the owner nods before flagged facts
land). Basis mapping: owner-ruled → confirmed-by, everything else →
inferred (fleet's permission model: confirmed-by refuses supersede
without approval). user/feedback types are fleet PROTECTED_TYPES —
they survive curation folds.

Sequencing: the tree's 26+1 uncommitted memories/ churn commits first;
pilot = one fact per type through the DEPLOYED binary; batches in date
order, one chore(memories) commit each; recall proof = five real
instructions rank transplanted facts in the top ten.

**Why:** the owner's goal is fleet's recall carrying the same lessons
this harness holds; the census record (not memory) is the source of
truth for dispositions.

**How to apply:** the plan is ACTIVE (2026-11-04 — the owner's /goal:
"implement the desk and then the transplant plans"; the desk's Rust
core landed first, the transplant is next). The tree's 26+1 memories/
churn already committed (59f2602, chore(memories)) — the clean base
exists. Build the census before anything lands; never write a fact by
hand — only through the writer. See also
[[knowledge-work-core-named-desk]].
