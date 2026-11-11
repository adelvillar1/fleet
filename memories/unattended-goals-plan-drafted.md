---
name: unattended-goals-plan-drafted
description: "The unattended-goals plan COMPLETED 2026-11-05 (14/14, e293f04) — the main loop uncapped, the child bound config-authoritative with a re-dispatch ceiling, goals start runs and the goal tool completes them; the live proof found three defects"
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

**COMPLETED 2026-11-05, 14 of 14, pushed e293f04** (`docs/plans/2026-11-05-unattended-goals.md`,
recap `docs/recaps/2026-11-05-unattended-goals.md`). The plan went
through three owner rulings on its way from draft to done, all recorded
in [[the-cap-leaves-the-main-session]]: the cap leaves the main session
entirely; the parent must detect non-delivering children and absorb or
respawn; and the sys1-abstraction ruling
[[sys1-is-the-abstraction-a-vendor-key-gate-is-not]] landed alongside as
its own fix (95dc98c).

**What shipped.** `AgentOptions.capped` (false on the main path, true for
a delegated child) makes the loop uncapped on main and bounded on the
branch; `delegation.max_turns` replaces `runs.max_turns` (config
authoritative — the clamp lifted; the retired `runs` section still
parses); `MAX_NODE_REDISPATCHES = 2` enforced in `Graph::dispatch` and
persisted; setting a goal on an idle non-room conversation starts the
run; the `goal` tool (show/complete/clear) writes the completion to
`sessions-goals-done.json`, the state document exposes `goal_done`, and
the chip wears `goal · done`; supervision dispatches the bound node on
spawn and marks it on a non-delivering end, naming absorb / respawn /
surface and the attempt count; the graph card's revision line carries the
reason.

**The live proof found three defects the unit tests could not** (all
fixed and filed, KI-448/449/450):
1. **A deadlock that wedged the whole face** — the goal-start path read
   the bridge state through a guard held across `run_with`, which locks
   the same mutex, so every endpoint stopped answering. Fix: read into
   locals, release, then call.
2. **The goal tool keyed on the wrong id** — `options.session_id` is the
   wire-header UUID derived from the conversation, not the raw id, so a
   real completion answered "no goal is set". Fix: `goal_session`.
3. **The start was gated on the launch session**, so a goal on any other
   conversation silently did nothing. Fix: any non-room session with no
   run in flight starts (rooms drive their own turns).

Proof: `/goal` returned `started: true`; the run drove **112 turns** past
the old 100 cap and ended naturally; the tool answered
`{"completed":true,…}` and the state read `goal_done`; a delegated child
ended `max-turns` at its own 2-turn bound. Gate on the pushed tree: fmt 0,
clippy 0, release 0, lib 1,883/0, integration 166/0, harness 914/99.

**Still open:** `src/extract.rs` calls the System One vendors directly
(Fastino GLiNER for LOCATE, glide/drex/jev for VERIFY) — the last
violation of the sys1 ruling, plan-sized, awaiting the owner's go.

**CORRECTED 2026-11-06:** the `delegation.max_turns` half of "What
shipped" did not reach the child — the setting is read but its only
consumers are the state document's `turn_budget` and the main run's
unused `max_turns`; the child's cap is the call's `max-turns` or the
constant 20. C2's "config-authoritative" is a promise with no mechanism.
The owner's own words confirm the intent it violates: children "are
supposed to honor the cap". The CLI `--max-turns` is dead the same way
but is NOT a defect — the owner ruled the main-session cap dead on
request. See [[delegation-during-goal-execution-deep-dive]].

**How to apply.** The main session is uncapped: never reintroduce a turn
bound there; put bounds on branches. The absorb-or-respawn choice is the
model's, the transitions are machinery. See
[[a-python-literal-sweep-eats-non-literal-sites]] for the `AgentOptions`
sweep trap and [[a-plan-never-ticked-is-not-a-status]] for the discipline
this plan first broke (landed commits, criteria left unticked until the
owner asked).
