---
name: sessions-do-not-end-on-their-own
description: "Owner ruling 2026-10-27: sessions must not declare an end on their own — at minimum user-launched ones. Unless otherwise directed by the owner, sessions don't end. Pinned mechanism: 'completed' is stamped purely on a reply with no tool_calls (agent.rs:2390), never checked against anything."
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 43
superseded-by:
valid-until:
---

Owner ruling, 2026-10-27: "sessions shouldn't be able to declare an end on their own, or at least user launched sessions... unless otherwise directed by the user, sessions don't end."

**Why:** the harness's only stop test is reply shape. In the loop (src/agent.rs) a reply with no `tool_calls` field is stamped `Record::End { stop_reason: "completed" }` (agent.rs:2390) — nothing checks the goal, plan, or criteria; completion is taught in the system prompt ("keep working until the task is actually complete, then stop") but enforced by nothing. The only gate is the silent-stop repair (agent.rs:2349, `is_silent_stop`): an empty message or bare "done"/"ok" after tool work gets one re-ask. A *verbose false completion* sails through — the live proof is the dogfood triage session `20261027T135549Z-70d7a54d`, whose final message was a fluent handoff ("you restart Fleet once") and which the loop stamped `completed, turns: 4`. The bitter part: the system prompt's own rules make ending a turn with a question to the owner CORRECT — the run did the right thing, and the harness converted a correct turn-end into a session-end. Turn-end and session-end share one record and one word; that conflation is what the ruling targets.

**How to apply:** what the harness actually knows at a model stop is "the model handed back", not "complete" — the vocabulary should say so. Recommended shape (offered, not yet built): interactive sessions still write their end record (the tail read needs it to tell "loop stopped orderly" from "process died") but with a non-terminal stop reason (`handed back` / `awaiting owner`), and `work_state` (src/gui/bridge.rs:4172) renders it as an OPEN state in the owner's groups; "completed" becomes scheduled/delegated-only (bounded jobs, consumed reports — there it's true); true ends for user-launched sessions are only owner-directed stop (control file), max-turns, failure. "Completed without a report" currently ALSO renders as plain "completed" in work_state — same-family fix. Anti-Hermes compliant: the default changes, no new knob. Natural follow-on: with a declared goal (the goal chip), "completed" could earn itself back by tool-verifying the goal's criteria. The ruling's home is a plan/KI row, rideable by the parked fleet delegation once LayerFS is back. See [[sessions-rail-cap-keeps-oldest-200]], [[fleet-anti-hermes-constitution]], [[fleet-run-control-and-timeout-gaps]].
