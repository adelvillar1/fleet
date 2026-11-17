---
name: the-idle-sweep-must-never-disable-a-declared-tool
description: "The tool lifecycle sweep must never disable any tool the harness declares — protection is derived from tools::known_tool_names plus the capability tools plus every mcp__ lane tool, never a hand-list."
type: feedback
date: 2026-11-17
basis: confirmed-by
tier: durable
shape: [change, resolved]
phrases: [fleet tools enable vision, tools.disabled, PROTECTED_TOOLS, protected_native, is_protected, the lifecycle sweep disables only what the gate names]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

Owner ruling 2026-11-17: no core tool a harness uses may be disabled for being unused, no matter how long since its last use; an unused tool is not evidence it is unwanted.

The defect: lifecycle::PROTECTED_TOOLS was a hand-written list of ten names, so the idle sweep could disable — and had disabled on this home — nineteen of the harness's own tools (vision, deck, browse, ui, web, plan, process, extract, use_skill, list_dir, apply_patch, repo_map, file_change, convention, improve, frame, review_frame, hybrid_search, mcp_resource, mcp_prompt), plus five mcp__blend1__* lane tools. `vision` was swept 2026-10-31 for "idle — 0 success in 0 call over 50 declared runs" and stayed off 17 days, which is why the vision tool reported absent.

The fix, in src/lifecycle.rs: `protected_native()` derives the set from tools::known_tool_names() + capabilities::{MEASURE,IMAGINE,BROWSE}_TOOL + PROTECTED_TOOLS; `is_protected()` also protects every `mcp__` lane name because a lane's availability is already governed by the lane's own lifecycle. state_of() and both manual guards (main.rs tools_disable, gui/bridge.rs tool write) call is_protected, so `--force` remains the one escape hatch. tools::known_tool_names() was itself missing GOAL/UI/EXTRACT/AUTHOR_PERSONA/MESSAGE_ORCHESTRATOR — added.

Gates: lifecycle::tests::no_core_tool_is_ever_swept_for_being_unused and the_protected_set_is_derived_from_the_harness. `fleet tools sweep` now reports 0 disabled / 42 skipped; config tools.disabled is empty.

Fixtures that used deck/vision/recall as idle subjects were repointed to `fixture__*` names — a protected tool can no longer demonstrate a sweep.

Separate pre-existing bug found, not fixed: Client::routed sets base_url to String::new(), and api::chat builds format!("{}/chat/completions", base_url), so the vision tool always fails with "Bad URL: RelativeUrlWithoutBase". vision is declared and reachable but cannot serve until that is fixed.
