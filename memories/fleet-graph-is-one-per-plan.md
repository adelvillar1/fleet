---
name: fleet-graph-is-one-per-plan
description: "2026-10-19 owner ruling — ONE dependency graph per PLAN DOCUMENT, because the plan is already the contract; only APPROVED plans generate a graph (the four plan statuses map onto a graph lifecycle); supersedes per-root-session persistence and dissolves the cross-session dependency problem entirely"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's ruling on 2026-10-19, after the delegation deep dive and the
cross-graph question: "I think there is a better approach to the dependency
graph, it is one graph per plan document. The plan is already the contract,
why not tie the graph to it?"

**The repo already decided this — the ruling confirms an existing design
rather than inventing one.** `docs/plans/README.md` is explicit and its
rules are load-bearing:

> Plans are contracts. One file per body of work, named
> `YYYY-MM-DD-<slug>.md`
>
> The plan file is the source of truth for "is X done" — not the recap,
> not memory.
>
> `active → completed` when every acceptance criterion is confirmed met
>
> Sections: Context, Approach, **Acceptance Criteria (verifiable,
> specific)**, Files to be touched, Out of scope, Verification, Linked
> artifacts.
>
> Never reopen a completed plan — start a new one.
>
> Mid-implementation scope changes: update the plan's acceptance criteria
> first, then build.

So the plan is already the contract, already owns "is X done", and
already transitions to completed when every criterion is met. What it
lacks is an *execution record*. That is exactly what a graph is. The
ruling is the missing half of a design the plan format was already
reaching for.

## Only approved plans generate a graph

The owner's follow-up ruling: "correct, only approved plans get to
generate a graph." This maps the plan format's four statuses onto a graph
lifecycle with no extra states:

- `draft` → **no graph.** Nothing executes.
- `active` (approved) → the graph is generated from the declaration
  block. This is the *only* transition that creates one, and it is a pure
  function of an approved plan — deterministic and testable, with no
  partial states.
- `completed` → terminal and retained. "Never reopen a completed plan" is
  already the repo's rule, so this is naturally permanent — which is the
  traceability the owner asked for, satisfied by a rule that already
  exists.
- `cancelled` → retained, marked cancelled.

**This settles the authorship fork** that was open below: the human
approves the plan and the graph follows. The `plan` tool *drafts* the
declaration block into a plan document; approval is the gate; the graph
is generated from the approved artifact. No need to ask a model to emit a
well-formed DAG as an act of authorship rather than as a proposal.

## The shape that falls out

- **The plan carries the declaration.** A fenced block in the plan
  document declares nodes: id, prerequisites, cost, capabilities,
  approach, and — the mapping that makes it work — **which acceptance
  criterion each node serves**. Prose stays the contract for humans; the
  block is the contract for the harness. One file, so they cannot drift.
- **A sibling file carries the runtime state** — status, deltas,
  revisions, and a digest of the declaration it was built from —
  gitignored the way `memories/.recall-index.json` is a cache beside its
  corpus. The plan is git-tracked and human-edited; its execution state
  must not dirty the tree on every transition.
- **The acceptance criteria become the progress surface.** A node is
  verified when the criterion it serves is met, and the plan's own
  status rule already says `active → completed` when every criterion is
  confirmed met. So `fleet plan <slug>` can render the criteria table
  with each row marked met or unmet straight off the graph — the
  contract and its execution record in one view. More useful than a
  separate graph renderer.
- **"Files to be touched" already exists in the format** and maps
  directly onto the graph's `paths` field, so the resource edge — and
  the write-path lock that must come first — can be derived from a
  declaration the plan already makes, rather than invented at node
  granularity.

## What it resolves

**Persistence and scope.** One graph per plan, no clobbering between
sessions, terminal when the plan completes — and "never reopen a
completed plan" is already the repo's rule, so a completed plan's graph
is naturally a permanent record. This is the traceability the owner
asked for.

**The cross-session problem dissolves — this is the big one.** See
[[fleet-cross-session-dependency-problem]]: two sessions working one
plan are not two graphs needing linking, they are two *executors of one
graph*. No registry, no external-dependency machinery, no session
discovery. The problem I spent a turn designing around disappears
because the graph is scoped to the work, not to the process doing it.

**Cross-graph becomes cross-plan.** If plan B genuinely depends on plan
A's work, the identity is a plan *slug* — human-chosen, git-tracked,
stable — not a session UUID. Cross-plan dependencies are rare and
deliberate in a way cross-session ones never would be. This is where
`Unlock.unresolved` (currently a declaration error) gets used, and it is
a far smaller surface than was being scoped.

**Work with no plan has no graph.** A quick `fleet run "fix this typo"`
keeps today's flat, prose-ordered delegation exactly as it is. The graph
appears when there is a contract to execute.

**Demotion gets a natural home.** The repo already says "update the
plan's acceptance criteria first, then build" — which is exactly a node
going back to pending with a named reason. The rule exists; it just has
no execution record to apply it to.

## What it does NOT fix

Unchanged and still open from
[[fleet-delegation-architecture-deep-dive]]: the depth problem and
promotion; the score's sum-over-paths arithmetic
([[fleet-graph-score-double-counts-shared-deps]]); the write-path lock,
still first; the fact-gate's three checks; the unit-test attribution
problem (tests are global, criteria are local); and the declaration
block itself.

## New problems it creates

**Declaration drift, which is the NORMAL case not an edge case.** The
plan is git-tracked and human-edited while the runtime graph is keyed to
a declaration version — and the repo's own rule ("update the acceptance
criteria first, then build") means mid-run plan edits are *expected*.
Fix: a declaration digest in the runtime file, and a mismatch is named
rather than silently mismapped.

**The plan format needs enforcing before the graph can rely on it.**
Three existing plans do not follow the README — the 2026-10-18
memory-and-cache plan uses `Status:` rather than `status:` and carries no
`slug`; the delegation-steering plan has a prose status paragraph. If
the graph reads the specified format, non-conforming plans simply have
no graph, which is clean degradation — but it means the format must be
enforced first.

**Why:** the owner found a better scoping than the one I recommended,
and the reason it is better is that the plan document already carries
the identity, the contract, the acceptance criteria, the touched files,
the completion rule, and now the approval gate. Tying the graph to it
reuses all six instead of inventing parallels — and it deletes the
cross-session registry problem outright rather than solving it.

**How to apply:** when delegation graph work comes up, scope the graph to
the plan document, not to the session or the run, and generate it only
from an approved (`active`) plan. Read the plan's README for the format
contract before designing the declaration block. Do not build a session
registry or cross-graph edge machinery — the one-graph-per-plan ruling
removes the need. Keep the write-path lock first regardless. Related:
[[fleet-delegation-architecture-deep-dive]] for the deep dive and the
design direction this ruling supersedes part of,
[[fleet-cross-session-dependency-problem]] for the problem it dissolves,
and [[fleet-graph-score-double-counts-shared-deps]] for the arithmetic
that still needs settling.
