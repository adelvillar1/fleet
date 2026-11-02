---
name: usage-and-anomalies-plan-exists
description: "The dsh-superagent article's two surviving explorations became the gated plan docs/plans/2026-11-02-usage-in-calibration-and-stream-anomalies.md — usage fields into the calibration stream (Increment A) and the six-class deterministic anomaly pass (Increment B) — implementation not started."
type: design
date: 2026-11-02
basis: inferred
tier: durable
shape: [change, docs]
phrases: [super agent, router and worker sessions, delegation-first prompt, dsh-superagent, offloop article, scheduling experiment]
missing: [per-provider confirmation of which usage fields the five lanes actually return — Increment A's live verification leg, the implementation itself: the plan is status: draft, C0-C6 unbuilt]
recall-count: 0
superseded-by:
valid-until:
---

The offloop dsh-superagent article (https://offloop.org/learn/deepseek-super-agent-design) produced, after field-level verification, two explorations worth building; the owner ruled 2026-11-02 to plan them, and the plan exists: docs/plans/2026-11-02-usage-in-calibration-and-stream-anomalies.md (status: draft, plan-gate PASS — jev, 7/7 coverage, 1/1 scope, commit 2503bfc, sitting on the design-parity branch pending the fast-forward to master, KI-397).

Increment A: three optional serde-defaulted fields (cache-read, cache-creation, reasoning) on the usage struct (src/api.rs:491 — today it carries prompt/completion/total only and providers' cache fields are parsed and dropped), riding RouteRow (src/calib.rs:64 — no token field today) via the judged_from pattern; the calib stream surface renders the columns and a per-provider spend card. Increment B: src/anomalies.rs (new), six deterministic classes — fallback-walk, tool-failure-loop, lanes-down, delegation-maxed, run-failure-cluster, cache-collapse — each threshold a const; sys1 asked only to explain/triage findings that already fired (max two asks, zero on --dry-run, the beats' counting seam pins it); surface fleet calib --anomalies as cards. Rejected with reasons (event triggers, persistent workers, multi-repo scope) in the superseded fact's own body.

Two gate-infrastructure defects surfaced on the way, both ledgered: KI-395 (the write-guard refuses a document naming the established term's own canonical identifier, so plans go through bash) and KI-396 (the plan-gate's silent 2000-char per-section truncation — a coverage fix landing past the cap cannot move the verdict).
