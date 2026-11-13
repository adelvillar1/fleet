---
name: a-ruling-has-to-name-what-the-check-can-see
description: "feedback 2026-11-02 — O1's ruling described validating a relay join against \"the relay's own minted-session store\", but relay-sessions.json is the FACE's home file and the deployed relay cannot read it, so the check as ruled was unbuildable; the owner amended the ruling instead. Before building a ruled mechanism, verify the artifact it names exists on the side that would run it — otherwise bring the owner the amendment, not a substitute"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 56
superseded-by:
valid-until:
---

`2026-11-01-open-calls.md` ruling (a) for O1: *"Validate the join against
what the relay minted — the relay already persists its sessions
(`relay-sessions.json`, the KI-370 fix)"*. Read at build time, the
premise fails: that file is **the face's** store in its home, the relay
is a separate container behind OpenShip, and nothing carries the face's
store across. The alternative that shipped in 2026-11-01 (a shared
deployment join secret) was then flagged as "the ruling did not land as
ruled" (KI-399).

**Why:** a ruling is a decision about *what* should be true; it is not
a design. When its named mechanism is impossible, silently building the
nearest thing that works produces a divergence nobody ruled on — and a
later audit reads it as a dropped requirement. The honest move is to
surface the impossibility while still in planning, with the amendment
written out, and let the owner choose.

**How to apply:**
- Before building a ruled mechanism, check the artifact it names
  **exists where the check runs** and that the actor can reach it.
- If it cannot, stop and present the options (amend the ruling, build
  the missing infrastructure, leave it open) rather than substituting.
- Record the amendment in every place the ruling lives — the code's own
  header comment, the plan's ruling row, and the ledger row — so the
  three cannot drift.
- The same discipline as measuring a bound before setting it
  ([[instrumentation-must-measure-what-the-bound-bounds]]) and counting
  the ledger before recommending a defer
  ([[defer-is-a-counting-claim-check-the-bite-count]]).

**The sibling case — C18, 2026-11-02, RESOLVED.** The router's `composed`
leg looked unreachable the same way: `composed` is a real workload (its
own preference entry, its own blend1 candidate) but it was **not in
`JUDGED_WORKLOADS`** — the closed set the sys1 `route` task can answer —
so no live run could produce the verdict the criterion asked for. I
reported it as needing the owner's ruling. The owner's answer — *"you can
do it if I give you permission to do it, and you have it"* — was the
ruling, already given: the permission is the default, not the exception.
Both sides of the closed set moved (fleet's `JUDGED_WORKLOADS` and the
vendored lane's `ROUTE_WORKLOAD_LABELS`), and `tests/composed_live.rs`
proved it live — the real lane answered `composed` at 0.892 and the walk
landed on blend1. See [[composed-joins-the-judged-set]].

Related: [[corpus-contract-and-open-calls-plans-2026-11-01]],
[[e2e-keys-cannot-come-from-a-secret-the-intermediary-holds]],
[[a-relay-join-secret-lives-outside-the-dotenv]],
[[composed-joins-the-judged-set]],
[[open-work-closeout-2026-11-02]].
