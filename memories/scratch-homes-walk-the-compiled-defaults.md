---
name: scratch-homes-walk-the-compiled-defaults
description: "reference 2026-11-05 — a scratch FLEET_HOME with no routing.json walks the compiled default table (stepfun first, frozen at the 2026-10-16 migration); copy the owner's routing.json in before any live proof, and STEPFUN_API_KEY in ~/.fleet/.env keys the retired candidate"
type: reference
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

A live proof on a scratch `FLEET_HOME` does not walk what the owner's
home walks unless you give it the config. `load_routing` returns
`default_preferences()` on a missing `routing.json`
(`src/route.rs:1198-1206`), and that compiled table is frozen at the
2026-10-16 migration: `hard` opens with `("stepfun", "step-5-preview")`
(`route.rs:616-624`) — a provider retired from the owner's real chain.
A home table REPLACES each workload's chain wholesale
(`slot.candidates = candidates`, `route.rs:1228-1234`, no union), so
the owner's real walks never see stepfun (their `hard`: opencode →
ollama-cloud → xiaomi-mimo); only a config-less home does.

Two consequences hit in one proof (2026-11-05, the goal-pane live pass):

1. **Copy `~/.fleet/routing.json` into the scratch home before the
   first run.** The first attempt walked the compiled defaults, died at
   stepfun's account-state 400 (which the failure-class table does NOT
   fail over — `route.rs:2655-2675` classes 400/404 "client-error" so
   the walk stops before the deepseek tail; KI-451, open), and I only
   got a run by pinning deepseek — a workaround, not the proof the
   owner's chain was supposed to give.
2. **Sourcing `~/.fleet/.env` into the scratch server's env supplies
   `STEPFUN_API_KEY`**, which keys the retired candidate and makes it
   attemptable. Sourcing .env is right for the lane/model keys the
   proof needs; be deliberate about which roster keys ride along.

Refreshing the compiled fallback to match the owner's current chain is
an open owner call (it is the no-config default; the "compiled table
matches the kit's live one" note in CLAUDE.local.md is now stale).
See [[hard-chain-leader-walk-and-the-400-no-failover]].
