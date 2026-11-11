---
name: a-goal-set-must-start-the-run-and-a-completion-must-land
description: "feedback 2026-11-05 — a goal that only decorates prompts is a goal that does nothing; setting it must start the run, and its completion must reach the store the face reads (the live proof caught a state-guard deadlock, a UUID key, and a launch-session gate)"
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

The owner's report that drove the unattended-goals plan: *"setting a goal
to implement a plan doesn't do anything."* Verified in code: the goal
rode every run's **prompt** and nothing ever started a run from it — the
objective sat in the store until the owner happened to type a task.

**The class this names:** a stored intention that a surface displays but
no mechanism acts on is not a feature, it is decoration. When a control
writes state, something must consume that state — otherwise the owner's
press is a no-op wearing a confirmation.

**The three defects the live proof found**, each of which a unit test
could not see (KI-448/449/450):

1. **The state-guard deadlock.** The start path read the bridge's state
   fields through a `MutexGuard` held across `run_with`, which locks the
   same mutex — so setting a goal wedged the **entire face**: every
   endpoint after it timed out. A guard must never be held across a call
   that takes it again; read into locals, release, then call.
2. **The wrong key.** The `goal` tool keyed the goals store by
   `options.session_id`, which is the **wire-header UUID derived from**
   the conversation id, not the id itself — a real completion answered
   "no goal is set on this session". `AgentOptions` now carries
   `goal_session` (the raw id) for this.
3. **The wrong gate.** The start was conditioned on
   `session == self.launch()`, so a goal on any other conversation did
   nothing at all. The right condition is "any non-room conversation with
   no run in flight" — rooms drive their own turns.

**How to apply:** when building a control that stores state, find its
consumer in the same breath and drive it live end to end — store write →
run start → completion → the surface's read-back. Unit tests pin each
hop; only the live path proves the chain. And a completion must land
where the face reads (`goal_done` on the state document), never be
inferred from the goal's absence.

See [[a-declaration-is-not-an-execution]] (the same family: declared is
not executed) and [[unattended-goals-plan-drafted]].
