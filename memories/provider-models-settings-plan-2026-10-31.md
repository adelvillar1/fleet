---
name: provider-models-settings-plan-2026-10-31
description: "Plan drafted at docs/plans/2026-10-31-provider-models-settings.md (91b7db1, amended 81955e4) — provider+key discovers the model catalog AND its declared settings (context window, output ceiling), disabled_models off-list, primary/secondary workload surface"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Plan IMPLEMENTED 2026-10-31 — LANDED 33a5679 + the audit pins fc420a4, **status completed**. Gate: fmt 0, clippy 0, suite 1,854/0, harness 525 checks / 54 cases. Draft notes: from the owner's settings rethink plus the steer *"it should add all the models from the provider"* (which ruled the scope: one provider's add loads that provider's MODELS — discovery, not hand-typing).

The flow: provider form gains a key VALUE field (write-only, never echoed); the bridge writes it through the settings vault seam (`<ID>_API_KEY`, vault when passphrase present else ~/.fleet/.env — the seam at settings.rs:1490), then verifies by `GET {base}/v1/models` (every HTTP roster candidate already speaks OpenAI shape). Discovery writes `providers[id].models` as the catalog; `disabled_models` is the off list (default ON — new models usable without a visit); the roster load filters it out of `Candidate::models`. Workloads surface: routing.json's ordered chain IS primary(head)/secondary(rest) — UI marks it, no new file shape; picker is enabled-models-only; workloads outside the judged label set (vendored ROUTE_WORKLOAD_LABELS: quick/standard_code/frontend/hard/prose/deep_context) marked "unjudged".

Key facts verified while drafting: `settings_write("provider")` → `settings::set_provider` (bridge.rs:6870); the provider editor hand-types models as chips (app.js:15864) and takes key ENV NAME only (app.js:15860); `resolve_model_pin` parses "provider/model" strings (route.rs:1820); route-cache verdicts are label-level so preference writes need no invalidation.

Increments P0–P3 (key+verify, discovery+off-list, primary/secondary surface, harness/docs), criteria P1–P7. Out of scope: new judged labels (a sys1 task change), non-OpenAI discovery, lane-candidate discovery. Named hazard: bridge.rs/app.js/harness.mjs carry parallel-session work — diff at edit time ([[a-parallel-session-co-edits-your-file]]).

Amendment 3 (81955e4): declared settings ride discovery — `model_settings` per provider ({context_window, max_output_tokens}) read from listing fields (OpenRouter-style context_length) and ollama's POST /api/show (the source the compiled CONTEXT_WINDOWS table's own comment at route.rs:88 names as its gap); hand override wins; `context_window()` resolves config→table→None so the compaction budget follows with no change of its own; `max_tokens` sent ONLY when declared (api.rs carries no such field today — a wire change guarded by the editor's clear). P1 gains the read, new P2 makes it act, P3/P4 renumbered, criteria P5-P7 added / P8-P10 renumbered.

Landed 33a5679 + audit pins fc420a4. The completion audit's two uncovered clauses (P8 reorder-write, P9 enabled-only picker) got harness checks in fc420a4 — a criterion whose clause has no test is not checked, the same rule the verifier enforced on 2026-10-30 ([[completion-verifier-audits-tracker-and-clauses]]). Two traps during the build, both recorded as feedback: [[a-brace-matcher-corrupts-at-first-unmatched-brace]] (a hand-rolled literal patcher corrupted six files) and [[the-pane-card-is-live-the-board-pair-was-dead]] (the provider form first landed in dead code, and the harness's attribute selectors silently match nothing).

Close-out: recap at docs/recaps/2026-10-31-the-providers-models-settings.md (7824e21, pushed); the app reloaded and the new `provider-discover` op probed live ([[session-recap-2026-10-31-providers-models-settings]]). The FS passage is proposed, not applied.

**Later the same day:** the design canvas landed ([[design-canvas-native-plan-2026-10-31]]) and its two tools moved both tool-surface bounds — 10,000 definition tokens, 36 visible tools ([[a-declaration-budget-is-a-measurement-too]]). Nothing in this plan's discovery or settings legs changed.
