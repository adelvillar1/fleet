---
name: fleet-graph-score-double-counts-shared-deps
description: "2026-10-19 — the dependency graph's score sums over PATHS rather than over the closure, so a shared dependency is counted once per path that reaches it; deliberate and test-pinned, but it contradicts the module's own definition of the score and misorders the critical path. Also records the owner's stated recompute model (complete a node → zero its score → notify parents to recalculate their branch) and the open derived-vs-push question"
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

Found on 2026-10-19 answering the owner's "a node can have dependencies on
multiple nodes, to it needs to account for that."

Multiple prerequisites are fully supported everywhere it matters — declaration
(`Node::prereqs: Vec<String>`), unlock (every prereq must verify), and the
replan path. The gap is in `score_at`'s arithmetic.

**The score sums over paths, not over the closure.** `score_at` walks each
prereq and adds `cost(d) + score_at(d)`. In a diamond where `left` and `right`
both depend on `base`, the base's cost enters the head's score once per path
that reaches it. The test pins this deliberately and spells out the
arithmetic: `score(ship) = (3+2) + (4+2) = 11`.

That 11 is neither of the two things a score could mean. The module's own doc
calls it "the total remaining work in a node's dependency closure" — a closure
is a set, so that would be `base(2) + left(3) + right(4) = 9`, each node once.
Time to completion would be the longest path, `max(2+3, 2+4) = 6`. So it is a
deliberate, documented, test-pinned design choice that contradicts the
sentence directly above it — not an oversight, but a real inconsistency.

**Where it bites.** `critical_path` picks its head by highest score, so a node
at the convergence of two branches is credited with both branches' total work
as though it were sequential: the diamond (branches 3 and 4) scores 11, and a
single chain of costs 2 and 9 also scores 11 — but the diamond's real longest
path is 6 and the chain's is 11. The convergence node looks as critical as a
genuinely twice-as-long chain. The readiness bar's denominator (`peak`)
inherits the same inflation.

**Where it does not.** Unlock is correct either way — any non-zero score
blocks, and a shared unfinished dependency is non-zero under all three
formulas. Dispatch ordering is unaffected, which is the surprising part:
`dispatchable` sorts by `appears_in_scores`, and `appears_in` returns a
boolean ("does this node appear in that node's closure"), so each node is
counted once per *dependent*, never once per path.

**It gets worse under the design the owner has since ruled on.** A `plan` tool
declaring root-to-atomic produces wide graphs with heavy convergence — many
nodes sharing a foundation — and the inflation scales with convergence. Under
today's unwired graph this is cosmetic; under dependency-aware delegation it
would be the number the readiness bar and the critical path are read from.

**The fix, if wanted:** split the one number into two, which is the standard
critical-path-method split. Remaining work as a sum over the *set* of
unfinished nodes in the closure (the diamond's 9) drives progress and the
readiness bar; time to completion as the longest path (the diamond's 6) drives
the critical path. Both are cheap to compute from the same memoized walk, and
`score_zero` is unchanged since it only cares about zero versus non-zero.

## The owner's recompute model, and the open question

The owner's stated model: "I see the score sum as agent completes a tasks sets
dependency score to zero and notifies parent(s) of the work to recalculate the
dependency score for the branch they belong to."

**Half of that already works, and it is the important half.** The score is
*derived on every read*, not stored. `score_at` skips a verified dependency,
and every public entry point (`score`, `scores`, `unlock`, `critical_path`)
allocates a fresh memo and walks the closure. So when a node verifies, its
contribution is already zero and every dependent's score already reflects it on
the next read. The propagation the owner describes is implicit in the
derivation — there is no message to send.

**The other half — treating it as a push — is a real change with a real cost.**
A push model needs reverse edges (`Node` only carries forward `prereqs`, so a
dependents map must be built and maintained) and it makes scores *stored*
values needing invalidation rather than derived ones that cannot go stale.

Recommendation, still standing and unanswered by the owner: **do not store the
score.** The reason is the previous session's entire subject — fleet just spent
a day on caches that invalidated wrongly (KI-228's recall index keyed on file
metadata, KI-229's route cache adopting foreign verdicts). This graph already
has exactly one stored derived value, `peak`, and it already has the bug: it is
not serialized, so it is rebuilt from all-`Pending` adds on load and a cost
revised up then down reads back at the lower value. A stored score with push
invalidation is that same failure mode with a much larger blast radius, because
a stale score silently misorders dispatch rather than merely shrinking a
readiness bar.

Scale does not justify it either. `unlock` allocates a fresh cache per call and
`dispatchable` calls it per candidate, so a pass is roughly O(N²) — but a plan
decomposes into tens of nodes. At N=50 that is a few thousand operations per
turn.

**The diamond arithmetic is unchanged either way.** Push or pull, a shared
dependency is still counted once per path, because the score is defined
recursively over forward edges. That is a definition problem, not a computation
problem, and it needs the two-number split regardless of how the numbers get
recomputed.

**"Notify parents" as an event rather than as recalculation is a real
feature.** A parent that was blocked becomes dispatchable and something should
notice now rather than on the next pass. The plan already asks for it ("a node
that unlocks mid-flight starts without waiting for the wave to finish") and a
test pins it; the current mechanism is that `dispatchable` is re-consulted. If
a genuine event is wanted it must feed into the wave planner rather than bypass
it, because the concurrency bound and the one-writer discipline exist precisely
to stop a completion from triggering an uncoordinated burst.

**The concrete change is now named** (from the 2026-10-19 exchange on how the
unlock signal is generated — see
[[fleet-unlock-signal-is-arithmetic-not-a-judgment]]): `plan_batch` computes a
wave plan once and executes it as fixed, so a completion that unblocks new work
cannot be honoured by an already-emitted plan. The fix is to make the wave plan
revisable — re-consult `dispatchable` after each completion and admit
newly-unlocked nodes into the remaining slots, still bounded and still
serialized on the resource edge. That is a contract change to one function in
`agent.rs`, and it is the piece that must exist before any scheduling logic is
reachable from a real run.

**Why:** the owner's one-line observation pointed at a real hole, and the hole
is not "multiple deps are unsupported" — they are — but that the one number
carrying them conflates three quantities. Anyone building the `plan` tool needs
this settled first, because the wider the declared graph, the more the
convergence inflation matters.

**How to apply:** when an arithmetic quirk is found, trace its consumers
before calling it a bug — that check is what split the verdict here into "two
of three consumers are wrong" rather than "the score is wrong," and it is why
dispatch ordering is exonerated. When the owner describes a caching or
recompute model, check first whether the derived-on-read version already gives
the behaviour before building the invalidation machinery — and name the prior
cache-invalidation defects as the reason not to. Related:
[[fleet-delegation-architecture-deep-dive]] for the deep dive and the owner's
design direction, which this finding affects, and
[[fleet-graph-is-one-per-plan]] for the scoping ruling that makes wide graphs
likely.
