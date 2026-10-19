---
status: completed
created: 2026-10-19
updated: 2026-10-19
slug: plan-tied-dependency-graph
---

# The plan-tied dependency graph

## Context

`src/graph.rs` is a complete, well-tested dependency graph with no
production driver. Its mutation and scheduling API is exercised only by
its own 37 tests and two `eval.rs` scenarios; `Graph::save` has exactly
one caller in the codebase, a test; and `<home>/graph.json` has never
been written by anything but a test. `fleet graph` renders a file no
production code produces, and the boards delegation panel shows a graph
that is always empty.

That is not the whole gap. Alongside it:

- **There is no task tree.** Delegation is flat — one `subagent` call,
  one child, one result. A parent expresses ordering only as English
  prose in the `goal` string, and the harness's only ordering guarantee
  is a writer-conflict wave plan.
- **There is no cross-process write coordination.** `write_file` takes
  no lock. Two fleet sessions editing one path race, last writer wins,
  and neither is told. The one-writer discipline the docs describe is
  per-turn and per-process.
- **The score sums over paths, not over the closure.** In a diamond the
  shared base is counted once per path (`score(ship) = (3+2)+(4+2) = 11`),
  which is neither the total remaining work in the closure (9) nor the
  longest path to completion (6). The module's own doc calls it "the
  total remaining work in a node's dependency closure," which is the
  first of those. `critical_path` picks its head by this number, so a
  convergence node is credited with both branches as though sequential.

The repo already owns the shape this should take. `docs/plans/README.md`
says plans are contracts, one file per body of work, with acceptance
criteria that are "verifiable, specific," and that "the plan file is the
source of truth for 'is X done'." What it lacks is an execution record.
That is what a graph is.

## Approach

**One graph per plan document, generated only from an approved plan.**
The plan is already the contract; the graph is its execution record.

- The plan carries a **declaration block** — a fenced block declaring
  nodes: id, prerequisites, cost, capabilities, approach, and the
  acceptance criterion each node serves. The prose stays the contract
  for humans; the block is the contract for the harness. One file, so
  they cannot drift.
- A **sibling runtime file** carries status, deltas, revisions, and a
  digest of the declaration it was built from. Gitignored the way
  `memories/.recall-index.json` is a cache beside its corpus: the plan
  is git-tracked and human-edited, and its execution state must not
  dirty the tree on every transition.
- **Only an approved plan generates a graph.** `draft` → nothing
  executes. `active` → the graph is generated, as a pure function of the
  approved plan. `completed` and `cancelled` → the graph is terminal and
  retained. The repo's own rule ("never reopen a completed plan") makes
  permanence the natural behaviour, which is the traceability
  requirement.
- **The acceptance criteria become the progress surface.** A node is
  verified when the criterion it serves is met, and the plan's status
  rule already says `active → completed` when every criterion is
  confirmed met. So `fleet plan <slug>` renders the criteria table with
  each row marked met or unmet straight off the graph.

**The graph is the work-breakdown structure; there is no separate
tree.** Work depth and session depth become different things. `Chain`
stays as a runtime record of who spawned whom — the approval prompt and
the delegation rows need it — but it stops being the work structure.
This is what makes the depth problem solvable: a node at work-depth 5
can be executed by a session at depth 1.

**Promotion, not a deeper hierarchy.** A child that discovers its node
is not atomic hands back a sub-plan rather than doing the work or
failing. `SUBAGENT_MAX_DEPTH` stays at 2. The argument against raising
it is arithmetic: `turn_cap_for_depth` is `(20 >> depth-1).max(5)`, so
depth 3 and below get five turns. A four-level hierarchy whose bottom
two levels can barely act is not a hierarchy.

**Summary nodes.** The plan tool may declare a node as a summary — it
has children, no work of its own, no cost. A summary never reaches a
zero score until its children verify, so dispatch on "zero dependency
score, atomic regardless of level" holds without asking the orchestrator
to see N levels down on its first turn. This is a recommendation over
full up-front atomic declaration, and the tradeoff is recorded here for
the owner to overturn.

**Scores are derived on read, never stored.** Completing a node already
zeroes its contribution and drops every dependent's score automatically,
because `score_at` skips verified dependencies and every entry point
allocates a fresh memo. The propagation is implicit in the derivation.
A stored score with push invalidation is the failure mode this repo has
just spent a session on — the recall index keyed on file metadata, the
route cache adopting foreign verdicts — and the graph already has one
stored derived value, `peak`, which already has the bug: it is not
serialized, so a cost revised up then down reads back at the lower
value.

**The unlock signal is a function return, not an event, and never a
model call.** `is_unlocked` is three booleans over declared state.
sys1 exposes one tool, `sys1_classify`, a classifier; using it to learn
a boolean would cost ~1.2s per call (KI-044's measurement), be
non-deterministic, and invert the repo's own law — "Facts gate; reports
corroborate." sys1's legitimate role is the functional half of the
fact-gate, where a judgment is actually wanted.

**The fact-gate is three machine-checkable gates**, and it runs outside
the graph so the graph stays pure:

- **functional** — does the work integrate. A judgment; sys1 or the
  delegate beat.
- **unit tests** — does the suite pass. `cargo test`.
- **nomenclature** — are objects named per the guidelines.
  `nomenclature::check` (`src/nomenclature.rs:1206`).

Tests are global and nodes are local, so the full three-gate set runs at
**integration nodes** — those with dependents, where "does this
integrate" is the question — and leaves verify on functional plus
nomenclature. A leaf that composes with nothing has nothing to
integrate.

## Acceptance criteria

### P0 — the write-path lock (closes a live hole, independent of the rest)

- **C1** `write_file` takes an exclusive per-path lock through
  `src/filelock.rs::FileLock`, the same mechanism the memory corpus's
  recall-count bump already uses. Two fleet processes writing one path
  serialize; the second waits, it does not clobber.
- **C2** A lock that cannot be created leaves the write running rather
  than refusing it — the existing `FileLock::take` contract.
- **C3** The lock sibling is gitignored in the repo corpus, and a
  normal write leaves `git status` clean.
- **C4** Two concurrent writers to one path both land their content,
  in some order, with neither lost — pinned by a test that spawns two
  writers against one scratch path.

### P1 — the graph's own defects, before anything is wired

- **C1** `demote(id, reason)` is a deliberate transition: status back
  to `Pending`, the reason recorded as a `Revision`, dependents' scores
  re-raising automatically because the node is unfinished again.
- **C2** `dispatch` no longer demotes a `Verified` node — the current
  behaviour, which has no status guard and breaks the monotonicity
  claim, is replaced by `demote`. The monotonicity claim is restated
  correctly: the score only falls *as completions land*; a demotion is
  a named reversal.
- **C3** `save` is atomic — a unique temp file and a rename, the
  discipline every other writer in this crate runs on.
- **C4** A contradicted delta cannot rewrite the plan. `replan`'s
  falsified filter keys on `reason == Infeasible` and a non-empty
  `instead` today and ignores `contradiction`, so the one category that
  means "don't trust this" is trusted to replace a real approach.
- **C5** `record_delta` is idempotent per node, so a second report
  cannot lower `diff_score` and un-fire a replan the doc claims is
  "monotone by construction."
- **C6** `REPLAN_CAP` is enforced on `load`, so a hand-edited file
  carrying three revisions cannot permanently deaden replanning.
- **C7** `peak` survives a save/load round trip, either by serializing
  it or by recording the declaration digest it was computed at.
- **C8** A terminal all-blocked graph is named rather than silent: when
  every remaining node is blocked by failed or unresolved
  prerequisites, `ready()` and the render say so instead of returning
  empty.

### P2 — the score split

The fixture this is pinned against is the module's own diamond:
`base` cost 2, `left` cost 3 on base, `right` cost 4 on base, `ship`
cost 6 on left and right. Today `score(ship) = (3+2) + (4+2) = 11`.

- **C1** Two numbers, both from the same memoized walk. **Remaining
  work** is a sum over the *set* of unfinished nodes in the closure,
  each counted once — the diamond's `left(3) + right(4) + base(2) = 9`.
  **Time to completion** is the longest chain *below* the node, the
  critical-path method's earliest-start, excluding the node's own cost —
  the diamond's `max(3+2, 4+2) = 6`. The exclusion is stated because
  "longest path" is otherwise ambiguous: including the node's own cost
  would give 12, which is a third number and not the one scheduling
  wants.
- **C2** `score_zero` is unchanged — it only distinguishes zero from
  non-zero, and both new numbers are zero exactly when the closure is
  fully verified.
- **C3** The diamond is pinned at 9 for remaining work and 6 for time
  to completion, and the old 11 is gone. Every test that read the old
  score is updated in the same increment with its reason recorded.
- **C4** `critical_path` selects its head by time to completion, so a
  convergence node no longer beats a genuinely longer single chain —
  today a diamond of branches 3 and 4 (real longest path 6) scores the
  same 11 as a single chain of costs 2 and 9 (real longest path 11).
- **C5** The readiness bar's denominator is remaining work, so the
  displayed fraction is the fraction of the closure's work left.

### P3 — plan-tied persistence

- **C1** The plan format is enforced: YAML frontmatter with
  `status: draft|active|completed|cancelled`, `created`, `updated`,
  `slug`. A plan that does not conform has no graph — clean
  degradation, named rather than guessed at.
- **C2** The declaration block parses through `Graph::add`, so cycles,
  duplicate ids, and unknown prerequisites are refused by name at
  generation time, exactly as they are today at declaration.
- **C3** The runtime file is a gitignored sibling keyed to the plan
  slug, and it carries a digest of the declaration it was built from.
- **C4** A plan edited after its graph was generated is detected: the
  digest mismatch is named, with both versions, rather than silently
  mismapping nodes. This is the normal case, not an edge case — the
  repo's own rule is "update the plan's acceptance criteria first, then
  build."
- **C5** The four plan statuses map onto the graph lifecycle with no
  extra states, and only `active` generates one.
- **C6** A completed plan's graph is retained and readable, and
  `fleet graph <slug>` renders a past plan's graph.

### P4 — the plan tool, approval-gated

- **C1** The `plan` tool *drafts* the declaration block into a plan
  document for the owner to approve. It does not author a contract
  outright; approval is the gate and the approved artifact is the
  contract.
- **C2** A drafted plan is `draft` and generates no graph.
- **C3** `fleet plan <slug>` renders the plan's acceptance criteria
  with each marked met or unmet off the graph, and names the nodes
  serving each criterion.
- **C4** A criterion with no node serving it is named as unserved
  rather than silently unmet.
- **C5** Work with no plan has no graph, and delegation without a plan
  is byte-for-byte today's behaviour.

### P5 — the fact-gate

- **C1** A verify service runs the three gates outside the graph and
  feeds a new `mark_verified(id)`. The graph stays pure — no I/O inside
  `settle`'s predicate.
- **C2** The functional gate is a judgment: sys1 or the delegate beat,
  reporting on whether the node's work integrates.
- **C3** The unit-test gate runs the suite; the nomenclature gate calls
  `nomenclature::check`.
- **C4** The full three-gate set runs at integration nodes; leaves
  verify on functional plus nomenclature.
- **C5** Each verification is auditable — which gate passed, when, with
  what evidence — and lands a calibration row naming the node, the
  criterion, and the gates that ran.

### P6 — dispatch from readiness

- **C1** `plan_batch` becomes revisable mid-flight: after each
  completion it re-consults `dispatchable` and admits newly-unlocked
  nodes into remaining slots, still bounded by `DELEGATION_CONCURRENCY`
  and still serialized on the resource edge.
- **C2** `subagent` calls reference a node id rather than carrying a
  free-form goal; the node's approach and capabilities become the
  brief.
- **C3** Dispatch selects nodes with a zero dependency score, at any
  work depth — the existing `dispatchable` rule, now reachable.
- **C4** Summary expansion: a child that finds its node is not atomic
  reports a fifth `DeltaReason` carrying child nodes, and the
  orchestrator merges the returned sub-plan into the graph with the
  returning node as their prerequisite.
- **C5** A node whose work does not integrate is demoted through P1's
  `demote`, with the reason named — the repo's existing rule
  ("update the acceptance criteria first, then build") finally has an
  execution record to apply it to.

### P7 — cross-plan dependencies

- **C1** `Unlock.unresolved` becomes a lookup rather than a refusal: a
  prerequisite that is not local resolves against another plan's graph,
  identified by plan slug.
- **C2** Cross-plan cycles are detected. Two plans blocking on each
  other deadlock today with no detector, and that is the nastiest
  failure in this design.
- **C3** A dependency on a dead or cancelled plan resolves explicitly:
  the dependent node fails or is released with a reason naming the
  dependency, never blocking forever.
- **C4** Cross-plan dependencies are rare and deliberate — the surface
  names both plans and the operator sees the coupling before it is
  declared.

## Files to be touched

**Graph core** — `src/graph.rs`, `src/graph/tests.rs`

**Scheduling and dispatch** — `src/agent.rs` (`plan_batch`), `src/subagents.rs` (node-referencing calls, the fifth delta reason), `src/tools.rs` (the `plan` tool, the `subagent` schema)

**Write path** — `src/agent.rs`'s `write_file` arm, `src/filelock.rs`

**Fact-gate** — `src/nomenclature.rs` (already has `check`), the delegate beat in `src/delegate.rs`, a new verify service

**Plans and surfaces** — `docs/plans/README.md` (format enforcement), `src/main.rs` (`fleet graph <slug>`, `fleet plan <slug>`), `src/boards.rs` and `src/gui/server.rs` (read the plan's graph, not a global one), `.gitignore` (the runtime sibling)

## Out of scope

- **A stored score with push invalidation.** Derived-on-read is correct
  and fast enough at plan scale. The reverse-edge index is the right
  optimization if plans ever reach hundreds of nodes, against a
  measurement rather than now.
- **Raising `SUBAGENT_MAX_DEPTH`.** The turn-cap arithmetic punishes
  depth and the plan's own evidence (587 spawn trees, none deeper than
  2) argues against it.
- **A separate task tree.** The graph is the work-breakdown structure.
  A second representation would drift.
- **sys1 generating the unlock signal.** It is a classifier; the signal
  is arithmetic. Its role is the functional half of the fact-gate.
- **Cross-machine or cross-process delegation.** The child remains an
  in-process `Agent` over an ordinary `Session`.
- **Automatic decomposition.** The declaration is the orchestrator's,
  drafted for approval; capability inference is the same fragile guess
  the delegation plan already refused.

## What landed on 2026-10-19, and what did not

**P0, P1, P2, P3, P4, P5 landed whole.** The gate runs green: `cargo fmt
--all --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --release`, `cargo test` — all four exit 0, 1244 passed, 0
failed. The lib went from 1065 to 1112 tests; two new modules
(`src/plan_graph.rs`, `src/verify.rs`) plus `src/recall_graph.rs` and
`src/filelock.rs` from the previous session's work.

**P6 and P7 landed whole.** C2 (a `subagent` call may name a declared node
instead of carrying a goal), C3 (dispatch selects zero-score nodes at any
work depth — `dispatchable`'s existing rule, now reachable through a plan
and pinned by a test), C1 (the admission is re-consulted after every
completion — `plan_graph::admit_now`), C5 (demotion, wired to
`fleet plan <plan> --demote <node> <reason>` and riding the runtime file
as a revision), and C4 (summary expansion: the fifth `DeltaReason`,
`not-atomic`, carrying the children its run found in `Delta::declares`,
merged by `plan_graph::merge_declared` — the node becomes a summary with
no cost and its children as its prerequisites, so it cannot start until
they verify).

P7: `Unlock.unresolved` became a lookup through
`plan_graph::PlanRegistry`, keyed by plan slug with `<slug>/<node>` as the
edge form. The registry scans the plan documents rather than the runtime
files, so an approved plan is resolvable without anybody having run it
first. A cross-plan edge that resolves to a verified node clears, one
that resolves to a waiting node holds and names the plan and node, and
one that can never clear is named in `Unlock::unresolvable` and in
`fleet plan`'s own output. Cross-plan cycles are detected by building the
combined namespaced adjacency — a cycle through another plan is invisible
from either side alone, so each graph's own check passes and the two
deadlock silently without this — and refused by name with the path.

**One defect found and fixed outside the plan's scope.** `Vocabulary::truncated`
conflated two different facts — "the tree exceeded a content bound" and
"the derivation ran out of wall-clock budget" — so a machine under load
reported this repository as not fitting inside its bounds. Growing the
suite from 1065 to 1112 tests raised the trip rate to three failures in
four runs, which is what surfaced it. The flag now carries only the
content fact, `budget_exhausted` carries the machine's, and the bound
test asserts the reporting rather than the machine's speed.

## Verification

The gate runs whole locally: `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
`cargo test` — all four exit 0. Per increment:

- P0 and P1 are pure defect fixes and are verified by new unit tests
  plus the existing suite, with no behaviour change elsewhere.
- P2 changes published numbers; the diamond test is rewritten to pin
  9 and 6, and every test that read the old score is updated in the
  same increment with its reason recorded.
- P3 and P4 are verified by round-trip tests: a plan generates a graph,
  the graph survives completion, a past plan's graph renders, and a
  plan edited mid-run is named rather than mismapped.
- P5 is verified by a fixture where a node's work does not integrate
  and the gate refuses it, and one where all three gates pass.
- P6 is verified by an eval scenario: a plan of six nodes with declared
  prerequisites dispatches in dependency order, a mid-flight completion
  admits a newly-unlocked node without waiting for the wave, and a
  summary node expands on demand.
- P7 is verified by two plans with a cross-plan edge, a cross-plan
  cycle that is refused, and a dependency on a cancelled plan that
  resolves explicitly.

## Linked artifacts

- `docs/plans/2026-10-14-fleet-delegation-steering.md` — the slice that
  built the graph core and deferred the dispatcher
- `docs/plans/2026-10-18-memory-and-cache-fixes.md` — the cache-
  invalidation findings that argue for derived-on-read scores
- `docs/features/subagents.md` — the delegation feature doc, currently
  stale on the depth cap, the admissible-scope enum, and concurrency
- `docs/known-issues.md` — KI-070 (the recursive agent loop), KI-076
  (no wall-clock cap), KI-209/KI-210 (approval classes across
  delegation depth)
