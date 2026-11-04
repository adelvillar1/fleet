---
name: fleet-tool-availability-gaps-2026-10-23
description: "assessment + PLAN 2026-10-23 after the tool-surface build — the tool-availability gaps, then four owner amendments in one sitting: MEASURE AND RETIRE (lifecycle as increment 1, bootstrapped from the archive), DISABLE-don't- delete, AUTO-disable by the harness itself, and the same lifecycle for skills and loops; 8 increments, 25 criteria, DRAFT awaiting approval"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Asked 2026-10-23, immediately after the tool surface shipped
([[fleet-tool-surface-analysis-2026-10-23]]), whether any area has obvious
gaps in tool availability. The answer: the gaps are mostly *reachability*,
not raw capability. Owner: "let's create a plan to address these issues" →
`docs/plans/2026-10-23-tool-availability-gaps.md`. Four amendments followed
in the same sitting. Was draft/no-code; **owner 2026-10-23 evening: "we are
implementing the tool use improvement plan"** — implementation is under way
(in another session), and the router already gained the agent fan-out
execution type (swarm) from the sibling judgment plan
([[fleet-judgment-beats-spawn-per-row]]).

**The gaps (why the plan exists):** `sdm1`/`gen1`/`browser` are installed
and health-checked but nothing spawns them (no `spec_for` outside
`lanes.rs`), so their tools are never declared — `sys1`/`sem1` are reachable
only because `tool_search` calls them, `blend1` because `web` wraps it.
Also: background processes unmanageable; no code intelligence; documents
and the SSRF-guarded fetcher are GUI-only; memory writes but there is no
`recall`; MCP resources/prompts unsupported; delegation has no lifecycle;
grep/glob/file-ops are shell-shaped; and nothing is ever disabled.

**The ledger of amendments — note how each superseded the previous:**

1. *Measure and retire* (after my plan only added ten tools) → a lifecycle
   as increment 1 ([[a-tool-surface-must-measure-and-retire]]).
2. *Disable, never delete* → `tools.disabled`, one `enable` restores it.
3. *"The harness may disable"* — since disable is not destructive the owner
   is fine with the harness acting, and "the same could probably apply to
   skills, workflows, and loops" → the sweep applies, and a `Surface`
   adapter covers four kinds.
4. (Earlier) *"we have embeddings at the conversation level"* → the archive
   bootstraps the first pass.

**Amendment 1 — measure and retire.** The log has no declaration list —
only a catalog *count* — so no usage rate exists; that missing denominator
is why the first "declared and never called" rule was removed. The plan
adds a `Declarations` record per run (names, count, tier); per-tool
economics (declared runs, calls, successes, errors, denials, tokens per
call, declaration tokens, age, state); **trial** under `tools.trial_runs`
10 / `tools.trial_days` 14; **candidate** = not protected,
≥`tools.disable_runs` 10 declared runs, ≥`tools.disable_days` 14 old, and a
success rate below `tools.idle_rate` 1% — or ≥10 calls at an 80% error rate
(broken, not unused). Protected: `bash`, `read_file`, `write_file`, `edit`,
`grep`, `glob`, `todo`, `remember`, `supersede`, `subagent`.

**Amendment 2 — disable, never delete.** A disabled tool is not declared,
absent from `tool_search`, and a call answers "disabled: … `fleet tools
enable <name>` restores it" rather than "unknown", so aliases, resumed
sessions and plans still resolve. Enable is one config write, effective
next run, no rebuild. The plan removes **no code**; the `improve`
`tool`-target proposal was dropped; C5 pins "disabling is the only
retirement".

**Amendment 3 — the harness acts, on four surfaces.** `fleet tools sweep
[--dry-run]` computes the gate and a daily `lifecycle-sweep` cron job runs
it with `--apply`; it writes `tools.disabled` itself. Guardrails for an
unattended actor: at most `tools.max_disable_per_sweep` 3 per sweep,
never protected/trial/disabled, every disable in the calibration stream
beside the config write, a failed write disables nothing, `--dry-run` first,
and a re-enabled thing is cooldown-protected for `tools.reactivate_days`
(else it oscillates against the sweep). **Auto-enable stays out**: coverage
suggests `enable`, the owner decides.

A `Surface` adapter (declared / used / disable / enable) drives the sweep,
the card, and coverage for:
- **tools** — `Declarations` + tool calls → `tools.disabled`.
- **skills** — `Skills` record + `use_skill` calls → `skills.disabled`;
  the SKILL.md file is **never moved or deleted** (the library is
  cross-harness), fleet just stops listing it; a role-named skill is
  protected because a role's contract names it.
- **loops** — cron's expected fires vs observed runs; `src/cron.rs` gains
  `runs`, `successes`, `last_success` beside `last_outcome`; disable is
  `enabled: false`; judged only after ≥3 expected fires so a monthly job is
  not mistaken for dead; `lifecycle-sweep` and the corpus-refresh jobs are
  protected (the sweep must not disable its own scheduler).
- **workflows** — the adapter reserves the kind; fleet has **no generic
  named-workflow registry**, so there is no unit to measure and none is
  invented. Plans/roles/deck layouts are not workflows.

**The archive bootstraps increment 1 (owner: "embeddings at the conversation
level… derive what we have used or needed", confirmed).** Three layers,
each labeled with its strength:
- **Used, exactly** — `Assistant.tool_calls` + `ToolResult`.
  `session_recall`'s synthetic fact per session carries its successful calls
  as `phrases`, but they are **per session, not per turn**, so turn-level
  coverage must embed owner turns itself.
- **Declared, estimated** — a per-tool `introduced` date backfilled from
  git, joined with the run-shape preconditions the log records (`Skills`,
  `Memory`, `LaneTools`, home for `improve`); labeled an estimate, replaced
  by the exact record after the first window. This is what finally makes
  "new" distinguishable from "unused".
- **Needed, review-only** — the log's misses (unknown names, alias
  recoveries, bash doing a native tool's work, `tool_search` "no tool
  fits") plus `fleet tools coverage`: owner turns embedded against the tool
  cards at the calibrated floor ([[a-fixture-corpus-flatters-a-retriever]]),
  sys1's `tool_route` as an optional second cut. Never a disable input;
  out of scope: inferring need from the assistant's own prose.

**The live baseline.** 30 sessions: `bash` 331, `write_file` 22,
`remember` 18, `read_file` 17, `supersede` 2, `convention` 1; `deck`,
`vision`, `web`, `computer_use`, `plan`, `subagent`, `improve`, `use_skill`
zero — not a disable list (predates the record). The one actionable row is
`write_file` (18 errors in 22 calls) and it is protected: fix it.

**The rest (8 increments, 26 criteria).** 1 measure + auto-disable tools
(C0–C6, C21). 2 skills, loops AND workflows on the same seam (C22–C25). 3 capabilities
not lanes: `measure`→`sdm1_classify`, `imagine`→`gen1_imagine`, `browse`
one action-enum tool (open/read/screenshot) over a persistent browser.
4 process registry (output/wait/kill; a surviving child is **left alive and
named**; no PTY). 5 `apply_patch` (atomic per file) before any LSP + a
**heuristic** `repo_map`; bound 28→32 (repo_map and file_change named first
to defer). 6 `read_document` (local-first; `Escalate` is a named refusal —
no egress), `web` gains `url` through the SSRF-guarded `web::fetch`,
`send_file` + `Artifact` card. 7 `recall` (sessions only from cached
vectors) + `mcp_resource`/`mcp_prompt`. 8 grep context/type/multiline, glob
via `rg --files`, `file_change` (delete/move destructive, reserved paths).

**Lane facts the grounding corrected:** `gen1` exposes ONLY `gen1_imagine`
and answers **base64 data URIs, not a path** (decode and write; no
speech/transcription tool exists at all). `sdm1` is `sdm1_classify` only
(table + target + task_type), so `measure` mirrors that shape.

Out of scope: LSP/tree-sitter, background delegation (needs the parent's
approver + sink), PTY/stdin, ask_user/exit_plan_mode, gen1 speech,
notebook/PDF pagination, live vendor calibration, deleting a disabled
thing's code, auto-enable, auto-creating a promoted artifact, inferring need
from the assistant's own prose. Open knobs for the owner: the protected set,
the trial thresholds, the 1% idle rate, the per-sweep cap, and the promotion
similarity/threshold.

Related: [[a-tool-surface-must-measure-and-retire]],
[[harness-tool-surface-research-2026-10]],
[[embeddings-cannot-label-a-session-project]],
[[fleet-skills-invocation-is-model-only]] (the skills surface this now
covers), [[fleet-browser-lane-architecture]],
[[gui-web-pane-ssrf-guard]].
