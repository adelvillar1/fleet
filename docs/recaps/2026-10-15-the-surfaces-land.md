# 2026-10-15 — the surfaces land: roles, convention, the graph, the replan

Session: `20261015_090954_159ab2` · model step-5-preview (pinned stepfun)
Branch: `master`, pushed to `origin/master` at `c3957cb`.

## What the session was

The standing goal had two halves: fix routing using the enforcement
plan, then finish the delegation plan (slice 20). The routing half was
already complete when this session's work began — the enforcement port
(`d997557`), the blend1 lane path (`225b052`), the honest attribution
(`2812ca0`) — so this session finished the delegation plan's surfaces
and then its last core.

## What landed

**The router, proven before the surfaces.** The port's live leg closed
first: a real `fleet run` landed on blend1 through the composed
workload, the real lane answered a grounding ask citing KI-001/KI-002
from the indexed notes, and the route row carries the lane's own facts.
KI-191 (the sys1 crash's `",".join(provider)` on `None`), KI-202, KI-203
(the `start_index` order-position bug), KI-204/205/206 all closed or
logged with evidence.

**D3 — the role rides the delegation** (`0cabfad`, `021c09b`). Roles are
`<home>/roles/<name>.md`; `fleet roles` cards them; and the delegation
resolves the named role against the owner's home *before* anything is
created — an unknown role and a run with no home are named refusals the
parent reads. The role narrows the child's declarations to its admitted
set, its skills load beside the parent's, and its overlay rides the
child's prompt in a delimited frame. Two real bugs the new tests caught:
the narrowing base was `tools.unwrap_or_default()` — an empty set when
the call declared no tools, leaving the child with nothing — and
`system_prompt`'s plain-run early return skipped the overlay entirely.

**D7 — the convention tool** (`67612c9`). Every run now declares
`convention`, answering "what does this project already call this?" in
four states, each answered plainly: the established term with its
sites, established-as-written, a case-or-separator variant named as the
drift it is, and the honest miss as a first-class answer. `fleet
convention <concept>` is the CLI surface; the drift signal rides the
delegation evidence (the write-guard's flags read out of the child's
own log, named in the grading prompt); `nomenclature-guard` grades the
plan's six checks. One lesson recorded at both fixture sites: a code
file's string literals are identifier evidence for the vocabulary, so a
fixture naming an ordinary word retires some other proof's absence —
the first fixtures did exactly that to the honest-miss proof, and the
fix was non-colliding nonsense.

**D8 — the dispatch surface** (`589d7d0`). The graph persists as
`<home>/graph.json`: the state rides the file (a load that reset every
node to pending would render verified work as blocking work) and the
fact-gate deliberately does not (a verifier is the parent's closure and
cannot be written down). `fleet graph [--file <path>]` renders the
cards — the critical path, the readiness bars, the named holds — and
refuses by name with exit 2 when the file will not read.
`dependency-graph` grades the plan's checks, including the diamond's
pinned arithmetic (the shared base's cost enters the head's score
through both branches: 11, not 9 — the first fixture had it wrong).

**D10 — the replan the delta report drives** (this turn, uncommitted at
the time of writing). The node carries its delta; the graph derives the
aggregate diff score from them, weighted by reason category with a
forced deviation doubled and a contradicted report priced above every
category. Two named triggers — the aggregate at 4, a single node at 5 —
and the replan lands as an append-only revision naming its cause and
every change, revising the falsified approaches to what the child's own
run did instead and carrying the revision forward to the nodes waiting
behind them. A chosen improvement is a gain: it weighs nothing and
moves nothing. The render carries the score and each node's delta
reason. `delta-replan` grades the plan's six cases, the last through
the production seam (`bound_result` → `Delta::parse_render`) so a middle
node that summarized its children's deltas would fail the check rather
than pass silently. The cap is two revisions per run — the bound that
keeps the replan from thrashing.

## The gate

`cargo test` → 922 passed / 2 ignored across ten suites at each commit;
`cargo build --release` exit 0, warning-free, at each. rustfmt and
clippy remain uninstalled on this machine; the style was matched by
hand. CI remains disarmed by budget (owner decision).

## Live proofs

- `fleet convention judge_route` against this repo: the established
  term, 4 files through identifiers, real sites; a nonsense concept the
  honest miss; `JudgeRoute` the named variant; a bare call the usage
  refusal.
- `fleet graph --file <path>`: a three-node graph renders the critical
  path, the readiness bars, and the named holds; an absent file refuses
  by name and exits 2; a graph carrying a forced infeasible delta
  renders `diff score 6 of the 4 replan threshold · the replan has
  fired` with each node's delta reason on its card.

## What is left

Nothing in either plan's scope. Master is level with origin. The
routing plan's one named-open leg — a live run where the *real* sys1
judges `composed` (its route judgment answers low-confidence labels) —
remains a leg to walk when the lane's judgment sharpens, not a gap in
the port.
