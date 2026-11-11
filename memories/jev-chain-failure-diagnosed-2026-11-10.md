---
name: jev-chain-failure-diagnosed-2026-11-10
description: "reference 2026-11-10 — \"no provider answered (chain: jev)\" diagnosed end to end: the gates DO use the sys1 router (auto → route_decision) and the router ANSWERED [\"jev\"] reason=task — ~/.config/sys1/config.toml's by_task pins plan_gate/plan_surface_map to bare jev; jev then fails HTTP 402 (no TypeSafe credits); arch-gate passed because its pin is drex,jev"
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

The owner asked *"why use jev directly when you can use sys1 instead?"*
during the 2026-11-10 improvement-modes plan gates. Full diagnosis, all
read at source this session:

**The gates never bypass sys1.** `plan-gate`/`plan-surface` default to
`--provider auto`, which is `sys1.routing.route_decision(task, state,
cfg)` (`dev_decisions.py:2937`, fallback `or _sys1_chain("jev")` when
the router returns empty). A live probe through the REAL `load_config`
(with `~/.config/dev-decisions/env` sourced) answered:
`router would answer: ['jev']  reason: task`.

**`reason: task` = a deliberate pin** in `~/.config/sys1/config.toml`
`[providers.route.by_task]`: `plan_gate = "jev"`,
`plan_surface_map = "jev"` (header comment: *"Fan-out tasks are
Jev-native: glide times out on multi-head requests over ~33k-char
states (measured 2026-10-02)"*; `plan_surface_map` because *"jev won
both on the 2026-10-02 probe"*). A by_task override REPLACES the
default chain `glide,drex,jev` — so there is no glide/drex leg to fall
through; the chain prints as a single `jev` because the pin made it
one. (Earlier in the session I misread this as availability-filtered —
that was a raw-tomllib probe bypassing `load_config`'s sys1-provider
inheritance; the real cfg makes all five providers available:
decide/drex/glide/jev/liquid.)

**jev itself is billing-dead.** A POST to the wire's exact URL
(`jev_api_url` = `https://api.typesafe.ai/v1/systemone`, posted AS-IS —
no `/chat/completions` suffix; my first 404 was my own path guess)
returned **HTTP 402: "Your organization has no available TypeSafe API
credits"** (console.typesafe.ai/settings/billing).

**The contrast that proves it:** `arch-gate` PASSED the same session
because its pin is `arch_gate = "drex,jev"` — drex answered (keys load
fine in-process even from a keyless shell; the CLI sources
`~/.config/dev-decisions/env` internally). Same machine, healthy sys1
chain — only the bare-jev-pinned tasks starve. This also retro-explains
the failsafe plan's blocked plan-gate ([[failsafe-restore-plan-drafted-
2026-11-09]]'s `chain: jev`).

**Also recorded:** `plan-surface`'s structure preflight counts use-case
boxes too — **"15 criteria" = the 10 C-boxes + 5 use-case boxes** (the
checkbox-shift trap [[plan-gate-structure-and-association-traps]]);
plan_check/evidence-gate scope by section so completion isn't affected,
but note it at disposition.

**RESOLVED same session — the owner chose B:** `plan_gate`,
`plan_surface_map`, AND `plan_deps` re-pinned to `"drex,jev"` in
`~/.config/sys1/config.toml` (interim comment in the file: revert when
TypeSafe is credited; `pr_gate_fanout` left on jev — not our gates).
After the re-pin, both gates ran clean: plan-gate **coverage 10/10**
(dispositions: Reconcile-flag `overridden`, template-sections flag
`waived`), plan-surface artifact landed with suggested order =
checkbox order, arch-gate PASS. **Option A remains open as the proper
fix** — add TypeSafe credits (console.typesafe.ai/settings/billing)
and put jev back on its pins; until then plan-gate/plan-surface ride
drex,jev by the file's own revert note.

**How to apply:** when a gate prints `chain: jev`, don't guess provider
choice — read `~/.config/sys1/config.toml`'s `by_task` for the task id,
then probe the pinned provider's endpoint status; run gate probes with
the env file sourced and `load_config` (raw config.toml reads miss the
sys1 provider inheritance and misreport availability).
Related: [[improvement-modes-inbox-plan-drafted-2026-11-10]],
[[failsafe-restore-plan-drafted-2026-11-09]].
