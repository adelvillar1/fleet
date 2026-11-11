---
name: failsafe-restore-plan-drafted-2026-11-09
description: "project 2026-11-09/10 — the failsafe plan DRAFTED then LANDED through phases A–C (health loop bf79468, degradations d5711b4, repair note f982a64 on master); phase D (fleet recover, the local-model tail) is the in-flight uncommitted work; the plan file still reads status: active with 0/9 boxes ticked — the boxes lag the code"
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

**2026-11-10 update — the plan is no longer just drafted.** Its phases
A–C landed on master in three commits: `bf79468` (phase A, the health
loop — what is down, since when, the repair), `d5711b4` (phase B +
C3 — degradation over refusal at both child refusal sites, a broken
routing config degrading to the compiled-in order), `f982a64`
(C4/C5/C6 — the repair note riding the first request and the log, the
exhausted walk naming each cause and its repair). Phase D is the
**in-flight uncommitted work** in the tree at warmup: ~205 insertions
across `src/cli.rs`, `src/lib.rs`, `src/main.rs`, `src/route.rs`
(+108 — the local-model tail rung), `src/settings.rs` (+46 — the
last-known-good stamps) — i.e. `fleet recover` + C7 + C8. The plan
file still reads `status: active` with **0 of 9 boxes ticked** and
`updated: 2026-11-09` — the criteria lag the landed code; tick them
against the commits (the plan-check lesson: [[a-plan-never-ticked-is-not-a-status]]).
The recap of record for the resource contract notes the failsafe's
health loop shipped the same day.

The failsafe plan was **DRAFTED 2026-11-09** — see the ruling in
[[the-failsafe-restore-fleet-to-working-order]].
`docs/plans/2026-11-09-failsafe-restore.md` (status draft, uncommitted,
Approach split into `###` sub-sections under 2000 chars per the open
D-001 truncation defect).

Four phases, each standing alone: **A** the diagnostic loop (new
`src/health.rs` — tiered cheap probes: layerfs endpoint connect, lane
presence, API reachability; one repair map naming each service's repair
verb; gui-server interval, `/api/state` `services` block, face service
card); **B** degradation over refusal (the child's two refusal sites —
workspace open `subagents.rs:2216`, `prepare_child` `:2048` — become
degradations: plain-folder child / parent's route via the existing
`own_route = None` seam; main-session config refusal degrades to the
compiled-in chain order, loudly); **C** the repair attempt (a logged
repair note naming service/cause/verb; the repair is the main session's
own bash through the standing approval gate, one attempt per
down-transition); **D** the recovery mode (`fleet recover` — the
settings writer stamps last-known-good copies of config.json/routing
.json/fleet.env beside each write; improve undo, MCP last-enable
disable, the updater's `.prev` are the other layers' seams; the roster's
tail grows a FINAL rung behind the deepseek tail: a **configured local
model**, honoring the self-healing ultimate goal — see the ruling).

C0–C8, each with named verification; the live face (Docker store empty)
is the standing fixture for C0/C4. C7 = recover reverts a breaking
change (malformed routing.json, failing improvement) to last known good;
C8 = a configured local model serves when connectivity is gone, and
without one the failure text names the one unrecoverable failure.

**Gate status: plan_check.py green; plan-gate DID NOT run — its provider
chain answered nothing (`no provider answered (chain: jev)`), itself a
live instance of the failure class the plan makes survivable.**
**Diagnosed 2026-11-10** (see [[jev-chain-failure-diagnosed-2026-11-10]]):
the sys1 router's `by_task` pin sends `plan_gate` to bare jev
(`~/.config/sys1/config.toml`), and jev is billing-dead (HTTP 402, no
TypeSafe credits) — not a mystery chain, a pinned provider out of
credits; fix is credits (recommended) or re-pinning to `drex,jev`.
Next step: owner review → approval → gate run → implement. Related:
[[open-ki-ledger-deep-dive-2026-11-09]], [[layerfs-worker-dies-with-dockers-data]].
