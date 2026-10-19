# The plan becomes the graph's contract, and two increments the verifier caught

Status: complete
Date: 2026-10-19

Branch: `master`. This session carried two bodies of work. The first —
the memory-and-cache deep dive, its plan, and its implementation — is
recapped at `2026-10-18-memory-and-cache-fixes.md` and landed
uncommitted. This recap is the second: a deep dive into delegation and
the dependency graph, the design conversation that followed it, the plan
it produced, and the plan's whole implementation.

The arc: the owner asked for a deep dive on delegation, task tree,
dependencies, and dependency graph. The dive found that three of the
four exist and one does not — there is no task tree, delegation is flat,
and the dependency graph is a complete, well-tested component with no
production driver at all. What followed was a design conversation in
which the owner supplied the shape: one graph per plan document, because
the plan is already the contract.

## The dive found a graph with no driver

`src/graph.rs` is 1,984 lines with 37 tests, correct arithmetic, named
errors, a persistence format, and a replan loop. `Graph::save` has
exactly one caller in the codebase — a test — and `<home>/graph.json`
has never been written by anything but a test. `fleet graph` renders a
file no production code produces, and the boards delegation panel shows
a graph that is always empty.

Alongside it: **there is no task tree.** The `subagent` tool's schema has
seven properties and none of them expresses "B waits for A." A parent
expresses ordering only as English prose in the `goal` string, and the
harness's only ordering guarantee is a writer-conflict wave plan. And
**there is no cross-process write coordination** — `write_file` takes no
lock, so two sessions editing one path race with nothing to notice.

The fourth finding was the subtlest: **the score sums over paths, not
over the closure.** In the module's own diamond, the shared base's cost
enters the head's score once per path that reaches it —
`(3+2)+(4+2) = 11` — which is neither the work left (9) nor the time
left (6). `critical_path` picked its head by that number, so a
convergence node outranked a genuinely longer single chain.

## The owner supplied the shape

Five rulings, each of which resolved something the dive had raised:

**One graph per plan document.** `docs/plans/README.md` already says
plans are contracts, one file per body of work, with acceptance criteria
that are "verifiable, specific," and that "the plan file is the source
of truth for 'is X done'." What it lacked was an execution record. That
is what a graph is. This also dissolved the cross-session question
entirely: two sessions working one plan are not two graphs that need
linking — they are two executors of one graph.

**Only an approved plan generates one.** `draft` generates nothing,
`active` generates one as a pure function of the approved artifact,
`completed` and `cancelled` leave it terminal and retained. The repo's
own rule — never reopen a completed plan — makes permanence the natural
behaviour, which is the traceability requirement.

**Promotion, not a deeper hierarchy.** The depth problem dissolves once
work depth and session depth are different things: the graph carries the
work structure, `Chain` carries only who spawned whom. A node at
work-depth 5 can be executed by a session at depth 1. The argument
against raising `SUBAGENT_MAX_DEPTH` is arithmetic — `turn_cap_for_depth`
is `(20 >> depth-1).max(5)`, so depth 3 and below get five turns.

**Scores are derived on read, never stored.** A stored score with push
invalidation is the failure mode this repo has just spent a session on.
The graph already has one stored derived value, `peak`, and it already
has the bug.

**The unlock signal is a function return, and never a model call.**
`is_unlocked` is three booleans over declared state; sys1 exposes one
tool, a classifier. Using it would cost ~1.2s per call (KI-044's
measurement), be non-deterministic, and invert the repo's own law —
"Facts gate; reports corroborate." sys1's legitimate role is the
functional half of the fact-gate.

## The plan, and its whole implementation

`docs/plans/2026-10-19-plan-tied-dependency-graph.md` — eight
increments, 42 numbered criteria. All of them landed.

**P0** closes the only thing broken in production today: `write_file`
now takes an exclusive per-path lock through the existing `FileLock`.
Two concurrent writers to one path both land, neither torn.

**P1** fixes the graph's own defects: `demote` as a deliberate
transition, `dispatch` refused on a verified node (the accidental
demotion that broke monotonicity), atomic `save`, a contradicted delta
that can no longer rewrite the plan, idempotent `record_delta`,
`REPLAN_CAP` enforced on load, `peak` surviving the round trip, and
`is_terminally_blocked` naming a stuck graph rather than letting it read
as a finished one.

**P2** splits the score. `remaining_at` is now a true set sum and
`completion_at` is the longest chain. The diamond reads 9 and 6 where the
path sum read 11.

**P3** ties the graph to the plan: a fenced declaration block in the
document, a gitignored sibling runtime file carrying state plus a
declaration digest, drift detected and named, and the four plan statuses
mapped onto a graph lifecycle.

**P4** adds the `plan` tool — which drafts a declaration into a draft
plan and never touches its status, because approval is the owner's act —
and `fleet plan`, which renders the graph and the criteria roll-up.

**P5** builds the fact-gate outside the graph: functional, unit tests,
and nomenclature, with the full set at integration nodes and two at
leaves. Tests are global and nodes are local, so attributing a suite
failure to whichever node landed most recently is fragile; "does this
integrate" is the right question at a node with dependents.

**P6 C4** is the summary expansion the plan's own audit initially left
out. `DeltaReason::NotAtomic` is the fifth reason, and it carries a
payload the other four do not: `Delta::declares`, the children its run
found. `merge_declared` makes the node a summary — cost forced to zero —
and adds each child with the summary as its prerequisite, so the summary
cannot start until they verify.

**P7** makes cross-plan dependencies real. `Unlock.unresolved` became a
lookup through `PlanRegistry`, keyed by plan slug with `<slug>/<node>`
as the edge form. A cycle through another plan is invisible from either
side alone, so each graph's own check passes and the two deadlock
silently — the combined namespaced adjacency is what finds it.

## The gate

All four steps exit 0: `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
`cargo test` — **1252 passed, 0 failed**. The lib went from 1065 to 1121.

## Two bugs my own first drafts introduced

Both were caught by the tests written to pin the behaviour, which is the
argument for writing them.

**The set sum's guard was in the wrong place.** I put the `seen` check
inside the recursion while the cost was added at the call site, so a
shared dependency was still counted once per path. The diamond read 11
and the test that pinned 9 caught it.

**The merge's cycle check ran per-`add`.** A cycle between two children
is not visible until the second one is added, and by then the first has
already landed. The merge now rehearses the whole sub-plan against a
scratch graph before anything reaches the real one.

## One defect found outside the plan's scope

`Vocabulary::truncated` conflated two different facts — "the tree
exceeded a content bound" and "the derivation ran out of wall-clock
budget" — so a machine under load reported this repository as not
fitting inside its bounds. Growing the suite raised the trip rate to
three failures in four runs, which is what surfaced it. The flag now
carries only the content fact, `budget_exhausted` carries the machine's,
and the bound test asserts the reporting rather than the machine's
speed.

## What the verifier caught, and what it cost

The first implementation pass left two increments undone — P6 C4 and
all of P7 — and reported them honestly as deferred. The completion
verifier refused the objective anyway, and it was right: a green gate
covers what was built, not what was left out. Both landed in the
following pass. The lesson worth recording is that "deferred and named"
is a legitimate status in a recap and an illegitimate one in a delivery.

## Files changed

**Graph core** — `src/graph.rs` (+`src/graph/tests.rs`): the score split,
`demote`, `dispatch`'s guard, atomic `save`, `mark_verified`, the
declaration types, the cross-plan unlock and render, `add_prerequisites`

**Plan-tied graph** — `src/plan_graph.rs` (new): the declaration block,
the runtime file, drift, the criteria roll-up, `admit_now`, `demote`,
`merge_declared`, the registry, cross-plan cycle detection

**Fact-gate** — `src/verify.rs` (new): the three gates, integration vs
leaf, `verify_all`, the card

**Delegation** — `src/subagents.rs`: the fifth `DeltaReason`,
`Delta::declares`, the `node` argument on `subagent`

**Write path and surfaces** — `src/tools.rs` (the per-path lock, the
`plan` tool), `src/agent.rs` (the tool dispatch, the writer class),
`src/main.rs` and `src/cli.rs` (`fleet plan`, `--demote`, the cross-plan
report), `src/boards.rs` (the snapshot's second number), `src/calib.rs`
(the `verify` row)

**Cross-session** — `src/filelock.rs` (new, from the memory-and-cache
increment), `src/nomenclature.rs` (the split flag)

**Docs** — the plan, plus the four contract docs the memory-and-cache
increment corrected

## Doc updates applied

- `docs/plans/2026-10-19-plan-tied-dependency-graph.md` — the plan, now
  `status: completed`, with what landed and what did not recorded in it
- `CLAUDE.md` — the source map gains `plan_graph.rs`, `verify.rs`,
  `recall_graph.rs`, `filelock.rs`
- `.gitignore` — the per-path lock siblings and the plan runtime files

## Doc updates deferred (debt)

- `docs/features/subagents.md` — still documents `SUBAGENT_MAX_DEPTH = 1`
  against the code's 2, still lists an admissible-scope enum that omits
  `web`, and still says "**Not concurrent.**" **Deferred because**: it
  is the delegation feature doc and this increment did not touch
  delegation's behaviour, only the graph it dispatches from. **Target**:
  the next delegation-facing increment.
- `docs/features/` has no page for the plan-tied graph. The plan is the
  contract and it is complete; an operational reference is not.
  **Target**: alongside the subagents doc.

## Open questions / next steps

- **The wave loop does not yet call `admit_now`.** The mechanism is
  built and tested — re-consult `dispatchable` after every completion —
  but `execute_calls` still runs a wave plan computed once from the calls
  the model emitted. Wiring it is the one remaining seam between "the
  graph schedules" and "the graph is consulted."
- **The fact-gate's unit-test gate is disabled by default.** It shells
  out to `cargo test`, which is the one gate with a real cost, so the
  caller opts in. Nobody opts in yet.
- **Cross-plan edges are read-only.** A plan can wait on another plan's
  node, but nothing writes across the boundary.

## Notes

- `docs/plans/2026-10-19-adopt-on-contrast.md` is untracked in the tree
  and is not this session's. It was left untouched.
- The plan-tied graph and the memory recall graph from yesterday share a
  word and nothing else. They are unrelated subsystems.
- `tmp/` is untracked tool noise, not part of either increment.
