---
status: active
created: 2026-10-16
updated: 2026-10-16
slug: tui-routing-label
---

# Plan: the status bar names the routing mode, not the reserve

## Context

KI-217, the owner's report at the first launch after the voice round:
the status line above the message card showed `deepseek-chat` as the
model in use. That name is the compiled-in `DEFAULT_MODEL` seed
(`App::new(&config.model, …)`), and the bar renders it until the first
routed run reports a `route_how` — but fleet routes from its first
request, and DeepSeek is the pay-as-you-go reserve, declared in no
workload's chain, servable only on an owner pin. Naming the reserve as
"the model in use" is the exact misdirection the route surfacing exists
to prevent. A second defect rode beside it: once a run reported
`how = "pinned"`, the bar rendered "auto-router (pinned)" — the one
mode that is not the auto-router.

## Approach

The bar names the **routing mode** from launch, never the default
model:

1. At TUI startup the interface reads the home's routing state (the
   merged roster's pin) and sets the launch label: `auto-router`, or
   `pinned to <pin>` when the owner carries one.
2. The bar renders that label before the first routed run; after one,
   the `route_how` refines it — `auto-router (judged)`,
   `auto-router (fallback)`, or `pinned` (the parenthetical only when
   it adds information).
3. The model name renders only when no routing state is known at all —
   never in the TUI's own path.

## Acceptance criteria

- [ ] E1: at launch the bar shows `auto-router` (or the pin's label),
      never `deepseek-chat`.
- [ ] E2: a pinned run renders `pinned`, not `auto-router (pinned)`.
- [ ] E3: the gate runs whole, exit codes read directly.
- [ ] E4: KI-217 is in the ledger, closed with the fix.

## Files

`src/tui/mod.rs`, `src/tui/app.rs`, `src/tui/ui.rs`,
`docs/known-issues.md`, this plan.

## Verification

Render tests: the bar with a launch label and no run shows
`auto-router`; with a `pinned` `route_how` it shows `pinned` and not
`auto-router`; with neither, the model fallback survives (the honest
degradation). Gate per E3.
