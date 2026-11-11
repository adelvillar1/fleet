---
name: process-tiers-product-skills-memories
description: "Owner asked 2026-11-05 how much of fleet's development process is product-embedded vs skills vs memories — the three-tier map verified in code, and the recommendation (skill-ify the house lessons + seed project corpora) awaiting the owner's go"
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

The owner's question (2026-11-05): "how much of how we have been
developing fleet is embedded into the product process wise and how much
are just skills and memories? … I want to make sure that all projects
that I run through fleet itself are as thoroughly developed as fleet
has been."

The three-tier answer, each tier pinned in code this session:

**Tier 1 — product, rides every project automatically.** Plans-as-
contract is MECHANISM: the `plan` tool's own law — only an approved
plan generates a graph, and "a tool that could approve its own plan
would make the contract worthless" (tools.rs:96-98). The graph's three
real gates — two I/O, one judgment; the graph stays pure and takes
verdicts from outside; `mark_verified` promotes on fact-gate
confirmation (graph.rs:776). Plus the evidence spine (the desk), the
logged tool loop, approvals, compaction/context health, layerfs
checkpoints, `fleet ki append`, the calibration stream (judge/delegate
grading the day), `fleet improve` running the project's own gates, and
the lanes' floors / fail-open / named-absences.

**Tier 2 — skills, transfers to any project.** The ~459-skill
methodology body (plan-drafting, warmup, recaps, verify-fix-actually-
ran, regression-claim-verification, root-cause-first-debugging, the
mockup/design family) — now shape-routed by the C15 vocabulary.

**Tier 3 — memories + repo conventions, does NOT transfer
automatically, and this is the honest gap.** Verified in
resolve_root_from (memory.rs:734-781): the home corpus first (only if
it HOLDS facts — KI-087's foreign-ancestor fix), then the ancestor
walk from cwd — so a jobhound run recalls jobhound's corpus; fleet's
605 facts stay in fleet's repo. Tier 3 holds the house judgment
lessons (right-fixes-not-cheap-fixes, a-deferred-action-is-an-error,
say-only-what-you-have-read, the two-gates KI-410 lesson, the
load-sensitive class, census discipline, measure-AND-read, live proofs
on scratch homes) and the gate's four commands / mockups-first as
CLAUDE.md conventions.

**The owner already ruled the direction** (desk handoff brief, ruling
1): the methodology is integral to the harness, not project
scaffolding — process travels with the operator. Tier 1 embodies that
for code work and, since the desk, knowledge work.

**Recommendation given, awaiting the owner's go** (I offered to draft
it as a plan): both moves, in order — (1) skill-ify the house
judgment lessons as a `house-methodology` family with `shapes:`
declared, so shape-first discovery puts them in front of every run in
every project with zero new code; (2) seed each new project's corpus
with the house facts via the transplant procedure (census → writer
batches → recall proof), which is project-agnostic by construction.
The framing that landed: the skill carries the rules, the corpus seed
carries the scar tissue — the measured failures that make each rule
credible.

**Why:** the owner's goal — every project through fleet developed as
thoroughly as fleet itself — is one ruling and two mechanisms away;
the gap is precisely tier 3, and the mechanisms to close it both
exist.

**How to apply:** if the owner says go, draft the plan directly — no
re-derivation needed, cite this map; the criteria are skill files with
shapes plus a generalized seeding procedure (both mechanisms landed).
See also [[knowledge-work-core-named-desk]],
[[memory-transplant-plan-drafted]],
[[sys1-vendored-tasks-py-seam-and-the-python39-trap]].
