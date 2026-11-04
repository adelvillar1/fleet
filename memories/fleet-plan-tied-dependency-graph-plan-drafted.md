---
name: fleet-plan-tied-dependency-graph-plan-drafted
description: "2026-10-19 — the plan at docs/plans/2026-10-19-plan-tied-dependency-graph.md is IMPLEMENTED for P0-P6 (gate green, 1244 passed); P7 cross-plan dependencies and P6-C4 summary expansion did NOT land and are the named remaining gaps"
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

**Status: P0-P6 implemented 2026-10-19, gate green (1244 passed, 0 failed).
P7 and P6-C4 did not land. Nothing is committed.**

The plan at `docs/plans/2026-10-19-plan-tied-dependency-graph.md` was written
in the morning (status `draft`, no code) and implemented the same day after the
owner said `/goal implement plan as designed`. It now carries a "What landed on
2026-10-19, and what did not" section recording the split.

## What landed

- **P0 — the write-path lock.** `run_write_file` takes an exclusive per-path
  lock through the existing `src/filelock.rs`. Two concurrent writers to one
  path both land, neither torn. This closed the only thing broken in
  production: two sessions editing one path raced with no lock and no
  detection.
- **P1 — the graph's defects, all eight.** `demote` as a deliberate
  transition; `dispatch` refused on a verified node; atomic `save`; a
  contradicted delta can no longer rewrite the plan; `record_delta`
  idempotent; `REPLAN_CAP` enforced on load; `peak` survives the round trip;
  and `is_terminally_blocked` names a stuck graph.
- **P2 — the score split.** `remaining_at` is a true set sum, `completion_at`
  is the longest chain. The diamond reads 9 and 6 where the path sum read 11.
  Both numbers ride the render and the boards snapshot.
- **P3 — plan-tied persistence.** A fenced `graph` declaration block in the
  plan, a gitignored sibling runtime file with a declaration digest, drift
  detected and named, four statuses mapped onto a graph lifecycle, retention
  after completion.
- **P4 — the `plan` tool and `fleet plan`.** The tool drafts a declaration
  into a draft plan and never touches its status. `fleet plan <slug|path>`
  renders the graph and the criteria roll-up, generates when an approved
  plan has none, regenerates on drift, and names an unserved criterion.
- **P5 — the fact-gate.** `src/verify.rs` runs three gates outside the graph
  — functional (an injected reporter), unit tests, nomenclature — with the
  full set at integration nodes and two at leaves. `mark_verified` keeps the
  graph pure; every verification lands an auditable row.
- **P6, four of five.** C2 (a `subagent` call may name a declared node), C3
  (zero-score dispatch at any work depth), C1 (`plan_graph::admit_now`
  re-consults after every completion), C5 (demotion wired to
  `fleet plan --demote <node> <reason>`).

## What did NOT land — the named remaining gaps

- **P6 C4, summary expansion.** A child that discovers its node is not atomic
  still cannot hand back a sub-plan: `DeltaReason` has four categories and
  none means "this node is a summary, here are its children." The
  `summary: true` declaration flag is parsed and honoured (no cost, waits on
  its children) but nothing produces one. This is the single missing piece
  between "declares to a bounded depth" and "declares to atomic."
- **P7, cross-plan dependencies.** Not written at all. `Unlock.unresolved`
  is still a refusal rather than a lookup, so two plans cannot yet depend on
  each other. No cross-plan cycle detection, no dead-dependency resolution.

## A defect found outside the plan's scope

`Vocabulary::truncated` conflated two different facts — "the tree exceeded a
content bound" and "the derivation ran out of wall-clock budget" — so a
machine under load reported this repository as not fitting inside its bounds.
Growing the suite from 1065 to 1112 tests raised the trip rate to three
failures in four runs, which is what surfaced it. The flag now carries only
the content fact, `budget_exhausted` carries the machine's, and the bound
test asserts the reporting rather than the machine's speed. See
[[a-flag-that-conflates-a-tree-fact-with-a-machine-fact]].

## The plan's own conventions, still worth knowing

It conforms to the plan README's spec — YAML frontmatter with
`status`/`created`/`updated`/`slug` — rather than the looser style the last
few plans drifted into. That is deliberate: P3 is about *enforcing* that
format, so the plan conforms to it.

**Why:** the graph was a finished component with no integration point; this
plan specified the seam and then built it. Anyone picking up
dependency-aware delegation should start here rather than re-deriving the
design from the deep dive — and should know that the two gaps above are the
whole of what remains.

**How to apply:** the plan-tied graph is live and the gate is green, so
"wire `graph.rs` into the delegation path" is no longer the work. What
remains is summary expansion (P6 C4) and cross-plan dependencies (P7), both
specified in the plan's acceptance criteria. The write-path lock (P0) is
done. Related:
[[fleet-delegation-architecture-deep-dive]] for the diagnosis and design
direction, [[fleet-graph-is-one-per-plan]] for the scoping ruling the plan is
built on, [[fleet-graph-score-double-counts-shared-deps]] for the arithmetic
P2 fixed, and
[[fleet-unlock-signal-is-arithmetic-not-a-judgment]] for why sys1 is the
fact-gate's functional half and not the unlock signal's generator.
