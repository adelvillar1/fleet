---
name: delegation-during-goal-execution-deep-dive
description: "project 2026-11-06 — the goal run's delegation path traced end to end; two confirmed defects (delegation.max_turns never reaches the child, the subagent `node` parameter never resolves the node's approach/capabilities) plus three design questions; the CLI --max-turns is NOT a defect (the owner's own ruling), and the fix shape is config-as-ceiling with the call as a tightening; the owner's supervision nuance (check completions, kill unresponsive children, re-dispatch or take over) mapped against the code — o…"
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

The owner asked for a deep dive on **delegation during goal execution**
(2026-11-06). The path was traced in primary source, not inferred:

- A goal is session-scoped state in `sessions-goals.json`, keyed by the
  raw conversation id. `set_goal` on an idle non-room session starts the
  run with the goal text as the task (`bridge.rs:4293-4341`); every run
  carries it in the prompt's Goal section plus one fixed sentence telling
  the model to track work with `todo` (`agent.rs:2043-2053`). The `goal`
  tool (show/complete/clear) is declared on every homed run
  (`agent.rs:2307-2313`); `complete` moves the objective to
  `sessions-goals-done.json` and the dispatch site emits `GoalDone` +
  a `goal_done` record (`agent.rs:4212-4236`).
- A delegation is a second `Agent` over a second `Session`. The child's
  options are the crux: `capped: true`, `goal: None`, `goal_home: None`,
  `goal_session: None`, `memory: None`, `scope: Some(...)`, `ui_tx: None`
  (`subagents.rs:2182-2225`). The child gets the brief and nothing else —
  no standing objective, no goal tool, no memory, no lanes, no UI
  channel. Its cap is `request.max_turns` bounded by `inherit_turn_cap`
  (`subagents.rs:1946-1947`).

**Two confirmed defects** (both promise-vs-behaviour, the class the
open-ledger plan's C-group is about):

1. **`delegation.max_turns` does not bound the child — the setting is
   dead.** `run_config` reads it into `RunConfig.max_turns`
   (`route.rs:885-908`), but its only two consumers are the state
   document's `turn_budget` (`bridge.rs:954`) and the main run's
   `max_turns` option (`bridge.rs:11483,11541`) — unused, because the
   main loop is `capped: false` (`agent.rs:2995`). The child's cap is
   the `subagent` call's `max-turns` argument or the constant 20
   (`subagents.rs:1178-1193`); `subagents.rs` has never read `run_config`.
   Yet the settings summary says "the owner's value here is
   authoritative" (`settings.rs:131`), FUNCTIONAL-SPECIFICATIONS says
   the config is authoritative under the depth-stepped ceiling (`:468`),
   and the plan's C2 claims it is met. Only `docs/features/subagents.md`
   tells the truth (20 by default, 1–50 when asked) — the docs disagree
   with each other.
2. **The `subagent` call's `node` parameter does not resolve the node's
   approach or capabilities into the brief.** The spec promises "The
   node's own approach becomes the task and its capability declaration
   becomes the scope" (`tools.rs:177-180`), and the completed plan P6 C2
   says the same (`docs/plans/2026-10-19-plan-tied-dependency-graph.md`,
   "P6 and P7 landed whole"). The code sets the brief's task to the node
   **id** (`subagents.rs:1141-1150`); the module's own test admits it —
   "the node's id is the task until the node's approach is resolved"
   (`subagents.rs:4141-4144`). `dispatch_node` loads the graph but never
   reads `node.approach` or `node.capabilities` (`subagents.rs:1731-1759`).

**Three design questions, not defects without the owner's ruling:**

4. A delegated child's `todo` list is invisible to the goal's pane card.
   The child's `ui_tx` is `None` (`subagents.rs:2193`), so `emit` is a
   no-op (`agent.rs:5606-5610`); its `tasks` records land in its own log
   (`agent.rs:4205-4208`). The pane card rebuilds from the **parent's**
   log (`docs/features/gui.md:79-88`). During a goal run, work a child
   tracks never appears on the goal's task card.
5. The child can neither see nor complete the standing goal — by design
   (`goal: None` / `goal_home: None`, `goal` absent from
   `ADMISSIBLE_TOOLS`), matching the subagent spec ("sees nothing of this
   conversation"). The cost: a goal run that delegates must restate the
   objective in the brief's `context`, and nothing enforces that.
6. No delegation-count ceiling on the uncapped main loop.
   `MAX_NODE_REDISPATCHES = 2` bounds respawns of a *plan node*
   (`graph.rs:166,731-738`); an ordinary `subagent` call has no ceiling.
   The plan's Risks name this ("the pathological loop") and accept it.

**RETRACTED by the owner (2026-11-06): the CLI `--max-turns` is not a
defect.** The owner's words: "the cap of the main session is dead under my
request because it was stopping automated goal completion, children however
do have and are supposed to honor the cap." So a main-session cap's absence
is the ruling, not a mechanism failure — the CLI flag is the same main-session
construct, and the only residue there is the help text still advertising a cap
that no longer exists (a honesty nit, not a mechanism defect). The owner's
sentence also states the intent the child defect violates: **children are
supposed to honor the cap** — which is exactly what `delegation.max_turns`
fails to do.

**The fix shape (stated, not yet built).** The config is the child's
**ceiling**; the call's `max-turns` becomes a per-call **tightening** — it can
lower the ceiling, never raise it — with the depth-stepped ceiling still on
top. Mechanically: `SubagentRequest` must distinguish "absent" from "named"
(today `parse` collapses both into 20), and `run_subagent` must pass the run's
config value — `self.options.max_turns`, which the bridge already sets from
`run_config`. The `subagent` schema's `max-turns` description and the
settings/spec sentences then say what actually happens. The owner did not
rule on retiring the call's argument; the tightening reading is the default
taken unless they say otherwise.

**The supervision contract the owner added (2026-11-06).** The owner's
nuance: "parents have to actively check child process completions and are
responsible for killing unresponsive processes and either re-dispatching
the work or take over it." Mapped against the code, the cap is the
backstop, not the mechanism, and three duties are only partly met:

- **Check completions** — met by blocking, not polling: the parent's turn
  waits on the child (`agent.rs:5120-5197` → `subagents.rs:2298`), the
  outcome rides back as the tool result (`subagents.rs:2308-2344`), and
  the face sees Spawned/Working/Done (`subagents.rs:2024,2552,2429`). The
  parent's model cannot act while the child runs.
- **Kill an unresponsive child** — NOT met. There is no parent-facing
  kill. The owner's stop writes the **parent's** control file only
  (`bridge.rs:4481-4515` → `delegate.rs:2282-2303`); the child's control
  file is its own session's (`agent.rs:3745-3751`) and its watcher reads
  that (`agent.rs:2740-2764`). A stop refuses the child's approval-gated
  calls (the shared `GuiApprover` checks `stop_pending`,
  `bridge.rs:11169-11172`) but does **not** stop the child's loop — the
  child runs on to its cap or its return.
- **No wall-clock bound** — the cap bounds turns, not time: each model
  call can run 600 s (`api.rs:703,719`), so a 20-turn child can occupy
  the parent for hours.
- **Re-dispatch or take over** — met for node-bound children only
  (dispatch on spawn `subagents.rs:1842-1846`; mark failed
  `2352-2375`; ceiling 2 `graph.rs:166,731-738`; absorb/respawn/surface).
  An ordinary delegation during a goal run has none of that machinery —
  the parent's model decides from the tool result.

The fix scope proposed (not yet ruled): wire `delegation.max_turns` as
the child's ceiling, add a parent-facing child stop (write the child's
control file, and propagate the owner's stop to an in-flight child), and
add a wall-clock backstop. The owner was asked whether the nuance is a
correction to the framing or a work order, and which of the three gaps
to close.

**How to apply.** The two confirmed defects are the same family as
[[a-declaration-is-not-an-execution]] and
[[a-goal-set-must-start-the-run-and-a-completion-must-land]]: a declared
contract whose mechanism never runs. Before claiming a setting or a
parameter is authoritative, find its consumer and drive it live. The
open-ledger plan (`docs/plans/2026-11-05-open-ledger-closeout.md`) is
where these would be filed; its C-group is the promise-vs-behaviour
class. Nothing was written to the repo — the owner was offered the full
write-up and the KI row drafts.
