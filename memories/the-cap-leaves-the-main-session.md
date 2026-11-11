---
name: the-cap-leaves-the-main-session
description: "Owner ruling 2026-11-05 — the turn cap should not exist on a main session at all; context health is what matters, not turn count; delegation keeps its cap where it serves a technical purpose"
type: feedback
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

The owner's ruling (2026-11-05, reshaping the unattended-goals plan
mid-draft): "the cap shouldn't even be there on a main session... from
a user perspective the cap adds no value, even on a session that is
interactive, hitting the cap is counterproductive in spirit. If the
cap is a construct from a bad assumption that turn cap is used for
context preservation it needs to be reviewed, because context health
is what matters and not turn count. On the other hand if turn cap is
in fact useful during delegation then it should stay when justifiable,
and serves a technical purpose."

**Why:** the verification proved the ruling's instinct. The cap came
from slice 1 (84ba44a) as a cost guard — "an unbounded loop is a quota
fire, not an agent" (agent.rs:142-147) — never a context mechanism.
Context health (the fold, budget-following-model, survival check)
landed 2026-11-04 and does the real job. And delegation's cap IS
load-bearing: `inherit_turn_cap` (subagents.rs:140-150) depth-steps
the ceiling (depth 1 keeps SUBAGENT_MAX_TURNS, depth 2+ halves) and
ties a child to the parent's remaining budget, with the
uncapped-parent `None` path already built.

**How to apply:** the main session's loop is unbounded by turns — a
run ends on the model's report, the owner's stop, a refusal, or an
approval wait; the `max-turns` event and the cap receipt die. The
child bound moves to `delegation.max_turns` (config-authoritative,
owner-authored config is authoritative — "config can only lower" is
amended). The pathological-loop residual is the owner's accepted
decision: their stop is the bound. **Landed and proven 2026-11-05**
(e293f04): a live run drove **112 turns** past the old 100 cap and ended
naturally; a delegated child ended on its own 2-turn bound
(`{"stop_reason":"max-turns","turns":2}`).

**CORRECTED 2026-11-06 — the config half did not land.** The deep dive on
delegation during goal execution found that `delegation.max_turns` never
reaches the child: `run_config` reads it, but its only consumers are the
state document's `turn_budget` and the main run's `max_turns` (unused,
`capped: false`); the child's cap is the `subagent` call's `max-turns`
or the constant 20, and `subagents.rs` has never read `run_config`. The
claim "config-authoritative" in the settings summary,
FUNCTIONAL-SPECIFICATIONS, and the plan's C2 is a promise with no
mechanism — see [[delegation-during-goal-execution-deep-dive]]. The CLI
`--max-turns` is dead the same way, but that is NOT a defect: the owner
ruled the main-session cap dead on request ("it was stopping automated
goal completion"), and the flag is the same main-session construct — the
only residue is its help text still advertising a cap that no longer
exists. The owner's sentence also names the intent the child defect
violates: **children do have a cap and are supposed to honor it**, which
is exactly what `delegation.max_turns` fails to do. One build lesson: my
first cut removed the loop condition outright and uncapped the children
too — the child-cap integration test caught it, and the shape is a
`capped` flag (true only for the subagent's construction). The plan
carrying this is [[unattended-goals-plan-drafted]]; the context job's
machinery is [[context-health-research-2026-11-04]]; the retirement of the
turn count from surfaces was the earlier half-step
[[fold-count-is-the-freshness-measure]]; the deferred token-budget
question is dissolved by this ruling
[[turn-budget-shape-is-a-deferred-discussion]].

**The supervision half of the same ruling (2026-11-05):** "the main
loop is able to identify child delegation nodes that are not
delivering on their tasks — if a task is not being delivered it
becomes a failed node that either the parent needs to complete or the
parent needs to kill and fire up on a new node — or at least it should
do that." The verification agreed with the "at least": the graph
vocabulary is fully built (Status::{Pending, Running, Verified,
Failed}, `fail()` refused on verified, `demote(id, reason)` back to
Pending with a named Revision — the respawn path, `settle()` where a
fact outranks a failed report, `admit_now` re-consulted after every
completion, `unserved()` naming orphan criteria) — but the wiring is
NOT automatic: a capped/failed child renders its outcome as text for
the parent's model, node transitions left to it noticing, and a parent
could end its report with a failed node and blocked dependents
silently. The ruling lands in the plan as C12–C14: supervision is
MACHINERY (child spawn dispatches the bound node, child end marks it,
a per-node re-dispatch ceiling of 2), the absorb-or-respawn choice is
the MODEL's — the quota-fire law moves down a level: the main loop is
uncapped, a runaway branch is not.

**The nuance the owner added 2026-11-06** (restating the same ruling
during the goal-delegation deep dive): "parents have to actively check
child process completions and are responsible for killing unresponsive
processes and either re-dispatching the work or take over it." Mapped
against the code, the cap is the backstop, not the mechanism, and three
duties are only partly met — see
[[delegation-during-goal-execution-deep-dive]] for the full map. In
short: completions are met by the parent's turn BLOCKING on the child;
there is no parent-facing kill (the owner's stop writes the parent's
control file only, and does not stop a running child's loop); there is
no wall-clock bound (the cap bounds turns, each model call can run
600 s); and re-dispatch/take-over machinery exists for node-bound
children only, not for an ordinary delegation during a goal run. The
owner was asked whether this nuance is a correction to the framing or a
work order, and which of the three gaps to close.
