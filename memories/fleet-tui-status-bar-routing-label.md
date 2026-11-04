---
name: fleet-tui-status-bar-routing-label
description: "KI-217 (fixed 9514567, refined same day) — the TUI status bar names the routing mode from launch: `auto`, or a pin names its PROVIDER (`pinned · stepfun`, never the model); the deepseek-chat default never renders as the model in use"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner reported at first TUI launch (2026-10-16): the status line above the message card showed `deepseek-chat` as the model in use — the compiled-in `DEFAULT_MODEL` seed, i.e. the pay-as-you-go reserve, which serves nothing without an owner pin. Fixed in commit `9514567` (plan `docs/plans/2026-10-16-tui-routing-label.md`, KI-217): the interface reads the home's routing state at startup (`route::load_roster`'s pin) and sets `Status.routing`; the bar renders the launch label before any run and the route event refines it after; the default model never renders on the TUI path (the model fallback survives only for a state the TUI never constructs).

**Owner label refinement (same day, 2026-10-16):** the mode reads `auto` (not "auto-router"), and a pin names its PROVIDER — `pinned · stepfun`, never the model; a `provider/model` pair pin has the model half stripped ("the model is the router's business, the provider is the owner's"). Post-run, a pinned run renders `pinned · <provider that served>` (the Routed event's provider, previously set-but-unrendered); judged/fallback render `auto (judged)` / `auto (fallback)`.

**Why:** naming the reserve as "the model in use" is the exact misdirection route surfacing exists to prevent; the provider is the unit the owner thinks in, the model is the router's choice.

**How to apply:** any future TUI surfacing keeps the bar naming modes and providers, not endpoint/model names — the calibration route row is where the served candidate's detail lives. Same-day sibling fix: KI-216 (ac2d74e), loader skips render as one SkillsLoaded card instead of stderr over the pane. Related: [[orchestrator-routes-through-hard]], [[canonical-skills-library-agents-skills]].

**KI-255 (2026-10-20) is this law extended to every surface:** the GUI archive rendered `deepseek-chat` for scheduled runs step-5-preview served — the start record stamps the NOMINAL model (cron passes `DEFAULT_MODEL` pre-route) and no surface read KI-073's end-record `served_model`; the replay card even matched `served_model: None` and discarded the Some arm. Fixed: `SessionSummary::serving_model()` is the one rule (served model; nominal only on an ended unrouted run; null in flight) and archive/replay read it. The enforcement half: routing.json `gates.allow_api_models: false` keeps HTTP candidates out of every order (pin included — the gate outguns a pin). Same audit found KI-256 (cron's `lane_specs: None` — lane-named cron jobs can never succeed) and three deck-tool tests that had stacked `#[test]`s on one fn and never ran.
