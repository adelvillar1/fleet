---
status: draft
created: 2026-10-23
updated: 2026-10-23
slug: tool-availability-gaps
---

# Plan: the missing tool categories

## Goal

Close the capability gaps the surface audit found: three vendored lanes
with no model-facing path, process lifecycle without control, no code
intelligence, documents and web parsing that exist but are unreachable,
recall that is one-directional, MCP resources that are not spoken, and
the search/file operations that still fall back to shell.

It also gives the surface a lifecycle: every run records what it
declared, the advisor measures each tool's use and cost, new tools
start on trial, and the ones the evidence says do not pay for their
declaration are disabled, not deleted — the harness does the disabling
once the evidence clears, a future need is one `enable` away, and the
same seam covers skills, loops, and the router's workloads. The mirror
also holds: a work pattern that repeats is evidence for creating a
tool, skill, workflow, or loop, filed as a suggestion rather than
silently repeated. The first pass reads the existing archive — usage exactly, declarations approximately,
need as a review signal — so the lifecycle starts with the data already
on disk.

The audit behind it: `docs/architecture/tool-surface.md`, the
advisor's live scan, and `docs/features/tools.md`.

## Context

### What the model has today

Eight built-ins (`bash`, `read_file`, `write_file`, `list_dir`, `edit`,
`grep`, `glob`, `todo`), eleven beside-the-four tools (`use_skill`,
`remember`, `supersede`, `subagent`, `improve`, `convention`, `plan`,
`deck`, `vision`, `web`, `computer_use`), `tool_search` when the catalog
is tiered, and the enabled MCP servers' tools. The surface, the ledger,
the audit, and the lane-backed retrieval are in place.

### The gaps, with evidence

| Gap | Evidence | Consequence |
|---|---|---|
| `sdm1`, `gen1`, `browser` are unreachable | no `spec_for("sdm1"\|"gen1"\|"browser")` outside `src/lanes.rs`; their tools are never pre-joined or declared | no measurement, no image generation, no browser automation, despite the lanes being installed and health-checked |
| Background processes cannot be controlled | `run_bash_background` returns a pid and two file paths (`src/tools.rs`); no output/wait/kill/stdin tool | a dev server or build can be started but not waited on, fed, or stopped cleanly |
| No code intelligence | no LSP, symbol, repo-map, diagnostics, or `apply_patch` in `src/` | navigation is text search; multi-file refactors are many single edits; build feedback only via `bash` |
| Documents are parsed but not exposed | `src/documents.rs` is used only by `src/gui/bridge.rs` | the model cannot read an owner's PDF/DOCX/XLSX |
| The guarded fetcher is GUI-only | `src/web.rs::fetch` is used by the bridge; `web` goes through blend1 | a known URL cannot be read when blend1 is down |
| No artifact delivery | the deck is the only deliverable with a UI event | a file the model produced cannot be handed to the owner |
| Recall is one-directional | `remember`/`supersede` write; the memory block is injected; `memory::recall` is internal; `session_recall` is CLI-only | the model cannot ask "what do we know about X" or "which session did Y" |
| MCP is tools-only | `src/mcp.rs` speaks `initialize`/`tools/list`/`tools/call`; no `resources/*` or `prompts/*` | a server's documents and prompt templates are unreachable |
| Search and file ops are shell-shaped | `grep` has no context/type/multiline; `glob` skips known dirs but does not honor `.gitignore`; no move/delete/mkdir | the model shells out for what a tool should guard |
| Nothing is ever disabled | `fleet tools scan` counts calls and errors but not declared runs, age, or declaration cost; the earlier "declared and never called" advice was removed because it fired on tools added the day before; the log has no declaration record at all | a tool that costs declarations and is never used cannot be told apart from one added yesterday, so the surface only grows |
| The lifecycle stops at tools | `use_skill` calls and the `Skills` record are logged but no scan joins them; a cron job carries only `last_outcome`, with no run or success history (`src/cron.rs:645-665`); the router's workload chains are selected every run and recorded in `RouteRow`, but nothing ever retires one | an unused skill, a perpetually failing loop, or a workload nobody selects is invisible and stays forever |
| Repetition is invisible | the archive holds every session's task and its tool sequence, and the session index holds one vector per session, but nothing clusters them; the advisor counts per tool, not per pattern | the same work is redone by hand every week instead of becoming a skill, workflow, loop, or tool |

### Two corrections to the earlier assessment

- **`gen1` exposes only `gen1_imagine`** in this vendored copy
  (`services/gen1/service/mcp_server.py:63`), and it answers with
  base64 data URIs, not a file path (`:117-135`). Speech and
  transcription are not in the lane's tool surface; wiring them is a
  lane change, not a wrapper. The image tool must decode and write the
  bytes itself, reusing the base64 path the MCP image artifacts already
  use.
- **`sdm1` is not a general "measure":** `sdm1_classify` scores rows in
  a table with a target column (`classification`, `regression`,
  `forecast`, `anomaly`) and returns predictions with confidence. The
  capability mirrors that schema rather than inventing a broader one.

### The declaration budget

The native core is 19 declarations today. This plan adds ten (measure,
imagine, browse, process, apply_patch, repo_map, read_document,
send_file, recall, file_change) and folds `fetch_url` into `web` as a
`url` argument instead of a new tool. Twenty-nine native declarations
cross the current 28-tool bound, so the plan raises
`MAX_VISIBLE_TOOLS` to **32** — the low edge of the measured 30–50
degradation band — while keeping the MCP half deferred behind
`tool_search`. The Settings tools card reads the new bound.

The budget is a ceiling, not a target: the lifecycle increment below
measures which of these declarations earn their place.

### The disable decision has no ledger

The log records lane tools, skills, and a resolution row with a
catalog **count**; it does not record the declaration list. The advisor
can therefore count calls and errors, but not:

- how many runs declared a tool — the denominator of any usage rate;
- how old the tool is in this harness;
- what its declaration and each call cost;
- whether it is new enough to be on trial.

That missing denominator is why the earlier "declared and never
called" rule was removed: every tool added that day looked unused, so
the advice fired on new work and would have retired the surface it was
meant to curate. A disable rule without age and a trial window is
worse than none.

Three facts also make auto-removal wrong even with good data: a rare
tool can be decisive (`vision` on the one screenshot that matters), a
foundational tool can look quiet because a sibling covers it
(`write_file` for new files, `edit` for changes), and a tool can be a
fallback the model reaches for only when the primary fails. Retirement
is therefore a disable with evidence and the owner's decision: the
implementation stays, and a future need is one `enable` away.

### The first baseline, and why nothing is disabled yet

The live advisor over this machine's last 30 sessions (2026-10-23) has
one useful signal and one limit:

- calls: `bash` 331, `write_file` 22, `remember` 18, `read_file` 17,
  `supersede` 2, `convention` 1. `deck`, `vision`, `web`,
  `computer_use`, `plan`, `subagent`, `improve`, and `use_skill` had
  none in that window.
- the same window shows 117 `bash` calls doing grep/glob/sed work — an
  adoption signal for the tools that now exist, not a reason to disable
  anything.

The zero-call list is not a disable list. Those sessions predate the
declaration record, most predate the new tools, and several are
specialist or delivery tools whose value is decisive and rare. The one
row that already looks actionable is `write_file`: 18 errors in 22
calls. It is protected, so the right action is to fix it, not disable
it.

The archive still bootstraps the first pass: usage is exact, the
declared denominator is estimated from each tool's introduction date
and the run-shape preconditions the log records, and the coverage pass
ranks the owner turns whose intent was nearest a tool that was not
called. That is enough to review the current surface now; the exact
`Declarations` record takes over as the denominator after the first
window, and the first disable comes after the trial window, not
from this table.

## Approach

**D1 — capabilities, not lanes.** Every lane-backed tool is a native
declaration whose description says what it does, never a raw
`mcp__<lane>__<tool>` name. The lane joins on first call, so a run that
never uses measurement, image generation, or browsing spawns none of
them. The wrapper maps the model's arguments to the lane tool, calls it
through the roster's bounded seam, and names the lane, the provider and
the engine in the result.

**D2 — the capability table.** One wrapper per capability:

| Tool | Lane tool | Shape |
|---|---|---|
| `measure` | `sdm1_classify` | `table[]`, `target`, `task_type` enum, `model`, `num_estimators`, `table_name`; returns predictions with confidence/verdict |
| `imagine` | `gen1_imagine` | prompt, aspect ratio, count, provider, optional destination; the lane returns base64 data URIs, so the wrapper decodes and writes them and returns the paths, sizes, provider, and model |
| `browse` | `browser_navigate` / `browser_read` / `browser_screenshot` | `action` enum (`open`, `read`, `screenshot`), `url` for open, `selector` for read, `path`/`width`/`height` for screenshot; one persistent browser per run |

`browse` is one tool with an action enum rather than three tools: the
browser session persists across calls, and the declared surface stays
small. A missing lane (no key, no Chrome) is the lane's named reason on
the ordinary result path, never a silent empty answer.

**D3 — process control is a registry, not a shell trick.** `bash` with
`run_in_background` registers the child (id, pid, stdout/stderr file
paths, start stamp) in a per-run `ProcessRegistry` and returns the id.
`process` is one tool with an action enum:

- `output` — bytes from the last-read offset, plus whether the child
  is still running; incremental, so repeated calls do not re-read the
  file.
- `wait` — wait up to `timeout_ms`, then report completed or still
  running with the last output.
- `kill` — terminate and reap, reporting the exit status.

The registry lives on the `Agent` and is closed at run end. A process
still running when the run ends is **left alive and named in the final
report**, because killing a server the owner started is worse than
leaving it. A PTY/stdin mode is out of scope: `std::process` has no
portable PTY, and a pipe-only stdin would hang on anything that
prompts.

**D4 — `apply_patch` before an LSP.** `apply_patch` takes a
content-anchored patch (the OpenAI/V4A shape: begin/update/add/delete
sections, context lines, no line numbers) and applies it file by file.
Per file it is atomic: parse the whole patch, check every hunk against
the current text, write only when all hunks for that file match, and
refuse with the failing hunk and its nearest real line otherwise. It is
declared for every run, not only for OpenAI-class models: the evidence
is that content-anchored edits fail cleanly, and the tool is a
convenience over many `edit` calls, not a different safety model.

`repo_map` returns a token-budgeted map (default 1,000 tokens): files
ranked by how often their identifiers are referenced elsewhere, with
their top-level definitions. It is a **heuristic**, not tree-sitter:
per-language definition patterns (fn/def/class/struct/interface/const)
and an identifier-frequency rank. The result says it is heuristic. A
tree-sitter dependency and a real LSP are out of scope for this slice.

**D5 — documents first, egress never silent.** `read_document` calls
`documents::route` and `documents::extract` (local, bounded by the
module's own size cap) and returns the extracted markdown with the
parser named (`local:docx`, `local:pdftotext`, …). `Outcome::Escalate`
is a **named refusal**: it says the local parsers could not read the
format and that the owner's Documents surface is the escalation path.
The tool performs no egress; a model tool does not silently ship a
document to a parser vendor.

**D6 — the guarded fetcher is a tool.** `web` gains a `url` argument.
When `url` is present, the tool calls `web::fetch` (the same
SSRF-guarded fetcher the GUI panel uses), follows and re-validates
every redirect, and returns the final URL, status, content type, and
the readable text. When `url` is absent, the existing blend1 query path
is unchanged. This gives a known URL a direct, guarded read without a
new declaration.

**D7 — artifacts are a first-class delivery.** `send_file` takes a path
and an optional note, checks the path is inside the run's folder,
emits an `Artifact` UI event, and returns the path and size. The face
renders a card with the file name, size, and a download/open action.
The typed event is the record; the model's result names the same path.

**D8 — recall reads what the harness already holds.** `recall` takes an
instruction and returns the top facts from the run's memory corpus with
their scores and sources, plus matching skill names and descriptions
(lexical over the loaded library). Sessions are included **only from
the cached vectors** the session index already holds; a cold index is a
named "not warmed" answer rather than a minute-long embed inside a tool
call. Read-only, bounded, no model call.

**D9 — MCP resources and prompts.** The client gains
`resources/list`, `resources/read`, `prompts/list`, `prompts/get`, each
following `nextCursor` with a page bound. Two tools expose them:
`mcp_resource` (`list`/`read`) and `mcp_prompt` (`list`/`get`), with the
server argument validated against the joined servers and every result
bounded. A server that does not implement the capability says so by
name.

**D10 — search and file operations become guarded tools.** `grep`
gains `context` (lines before/after), `type` (a language/extension
filter), and `multiline` (rg only; the `grep` fallback names that it
cannot). `glob` prefers `rg --files` when rg is installed — which
honors `.gitignore` — and falls back to the native walker with a note.
`file_change` is one tool with an op enum (`move`, `copy`, `delete`,
`make_dir`): `delete` and `move` are in the destructive class and ask
in every mode, reserved paths (`.git`, the fleet home, the session
store) are refused by name, and `move` is a rename with a
same-filesystem fallback that is named when it copies.

**D11 — measure declarations, not just calls, and bootstrap from the
archive.** Every run writes one `Declarations` record before its first
request: the model-visible names, the count, and the tier. The advisor
joins that with the existing tool results to compute, per tool:
declared runs, calls, successes, errors, denials, tokens per call
(arguments plus result bytes over four), declaration tokens,
first-declared date, and age. A tool with no declaration history — an
MCP tool that joined mid-run — is dated from its first `lane_tools`
record. The scan window is configurable, default 50 sessions.

The first pass does not wait for the record to accumulate. It reads the
existing archive in three layers, each labeled with its strength:

- **Used, exactly.** Every `Assistant` `tool_calls` and its
  `ToolResult` — already what the advisor counts. The session index's
  synthetic facts carry their successful calls as phrases too, so the
  conversation-level embeddings already have a tool-usage handle.
- **Declared, estimated.** A static per-tool `introduced` date,
  backfilled once from the git history, joined with the run-shape
  preconditions the log already records: `Skills` for `use_skill`,
  `Memory` for `remember`/`supersede`, `LaneTools` for lane tools, the
  home's presence for `improve`. A tool is estimated declared in a
  session after its introduction date whose preconditions held. The
  estimate is labeled as one; the exact count starts with the record.
- **Needed, with evidence.** The log's own misses — unknown names,
  alias recoveries, bash commands doing work a native tool covers, and
  `tool_search` results that found nothing — plus an owner-intent
  coverage pass: embed the owner's turns and the tool cards with sem1
  and report turns whose nearest active card clears the calibrated
  floor but whose tool was not called. sys1's existing `tool_route`
  task can label each owner turn's capability class for a second cut.
  The coverage list is a **review** signal, never a disable rule.

The coverage pass is `fleet tools coverage [--sessions N]`: local-first
through sem1, class labels optional through sys1, cached by message
hash, bounded, and it generates no prose. It prints its method and its
limits beside every row, and it is opt-in because it reads the whole
archive.

**D12 — disable, never delete, and let the harness do it.** Disabling is
a config write, not a destructive action, so a sweep applies it instead
of asking the owner per candidate. `fleet tools sweep [--dry-run]`
computes the gate, and a `lifecycle-sweep` cron job runs it daily with
`--apply`; it writes `tools.disabled` itself. A disabled tool is not
declared, is excluded from `tool_search`, and a call to it returns a
named result — "disabled: no use over the last N declared runs;
`fleet tools enable <name>` restores it" — rather than "unknown".
`fleet tools enable` is one write away, effective on the next run. The
implementation and its tests stay exactly where they are: this plan
removes no code.

Each sweep disables at most `tools.max_disable_per_sweep` (default 3),
never a protected, trial, or already-disabled tool, and records every
disable in the calibration stream beside the config write. `--dry-run`
prints exactly what the next sweep will do. A disabled tool is not
forgotten: the coverage pass keeps watching it, and if owner turns
start matching it again the advisor suggests `fleet tools enable`
beside the evidence. Disabling is a decision the data can reverse, not
a deletion.

**D13 — the gate is a window, not a threshold.** A tool is a disable
candidate only when all of these hold:

- it is not in the protected set (`bash`, `read_file`, `write_file`,
  `edit`, `grep`, `glob`, `todo`, `remember`, `supersede`, `subagent`);
- it has been declared in at least `tools.disable_runs` (default 10)
  runs and is at least `tools.disable_days` (default 14) old;
- its success rate over the window is below `tools.idle_rate` (default
  1%) — including zero — or it has at least ten calls with an 80% error
  rate, which is a broken tool rather than an unused one.

A tool younger than `tools.trial_runs` (default 10) or
`tools.trial_days` (default 14) is **trial** and is never disabled. A
tool above the idle rate is active. The sweep may flag a protected or
frequently-denied tool for review, but it does not disable it; the
manual verb requires `--force` and names why.

**D14 — one lifecycle, four surfaces.** A `Surface` adapter answers four
questions for a kind: what was declared, what was used, how to disable,
how to enable. The sweep, the coverage pass, and the card are written
against the adapter, not against tools. The kinds and their sources:

- **tools** — declarations from `Declarations`, use from tool calls,
  disable via `tools.disabled`.
- **skills** — declarations from the `Skills` record, use from
  `use_skill` calls, disable via `skills.disabled`. The skill file is
  never moved or deleted; fleet stops listing it in the prompt and
  `use_skill` refuses it by name. A skill named by any role is
  protected, because a role's contract names it.
- **loops** — a cron job's expected fires over the window and its
  observed runs and outcomes; `src/cron.rs` gains `runs`, `successes`,
  and `last_success` beside `last_outcome`. Disable is
  `enabled: false`, enable is `fleet cron enable`. A loop is judged only
  after at least `loops.min_fires` (default 3) expected fires, and
  `lifecycle-sweep` plus the corpus-refresh jobs are protected.
- **workflows** — the router's named workload chains are the registry:
  `quick`, `standard_code`, `hard`, `prose`, `deep_context`, `vision`,
  and `composed` (`src/route.rs:378-445`). Declared = present in the
  preference/tier table; used = the `RouteRow` / `routed` selections in
  the calibration stream; disable = mask the label in the router's
  selection, so a verdict naming it falls back to the default and the
  row records `masked`; enable restores it. The default workload and
  `vision` are protected — the vision tool routes through `vision` — and
  `composed` is protected by policy because it is the grounded lane's
  only route. The mask is client-side: sys1's label set stays static.
  The execution topology (`single`/`mixture`/`swarm`) is the second
  workflow axis — sys1 already judges it, but `parse_workload` discards
  it today, so it joins this adapter when the judgment plan that reads
  it lands. Plan graphs are per-task declarations, not reusable
  workflows, and are not lifecycle units.

`fleet lifecycle sweep [--dry-run]` runs all four; `fleet tools sweep`,
`fleet skills sweep`, and `fleet loops sweep` run one. The cron job calls
the combined verb.

**D15 — promotion is the mirror of disable.** The same archive yields the
opposite suggestion: a pattern that repeats is a candidate for an
artifact. `fleet lifecycle promote [--sessions N]` clusters the archive
by the session's task vector — the session index already holds one
vector per session — and inspects each cluster's calls and timestamps.
A cluster is a candidate only after `promote.min_sessions` (default 3)
members and a similarity at or above `promote.similarity` (default
0.85); the thresholds are recorded with the suggestion.

- **loop** — the task recurs at a cadence: the cluster's start times
  cluster by hour and day. The suggestion prints the observed times and
  a `fleet cron add` line the owner can run; it is a loop because the
  work is the same on a schedule.
- **skill** — the cluster shares a tool-call sequence (a two-to-five
  call n-gram, arguments ignored) and the steps are advisory. The
  suggestion is a proposed `SKILL.md` through the existing `skill`
  proposal target.
- **workflow** — the cluster's sequence crosses stages: subagent calls,
  plan nodes, or an approval or handoff between steps. The suggestion
  is a plan-graph template, or a router workload when the pattern is a
  routing strategy; either way the proposal names the lane or config
  change it needs, because a new workload label is not a runtime write.
- **tool** — the cluster's work is shell commands no native tool
  covers, or owner turns whose nearest tool card was never called. The
  suggestion names the shape and files a `source` proposal.

Nothing is auto-created. The mirror of "the sweep disables" is "the
promotion pass proposes": tool and skill and workflow suggestions file
through `improve` with their existing targets, a loop suggestion is the
exact `fleet cron add` line, and the owner applies it. The same trial
window applies to the new artifact: a promoted tool or skill starts on
trial, so the disable gate cannot immediately undo the promotion.

## The app

The transcript's tool rows keep the resolution line and add the
capability's own facts:

```
✓ measure agents.csv · 812ms · sdm1 · tabpfn-hosted · 24 rows scored
✓ imagine a graphite icon · 6.2s · gen1 · qwen · assets/icon.png (412 KiB)
✓ browse read · 340ms · browser · 4,120 chars
✓ process wait · running after 10s · pid 4812 · 2.1 KiB new output
```

The artifact card on the conversation:

```
┌ assets/icon.png ──────────────────────────── 412 KiB ┐
│ a graphite icon with the serif f                     │
│ [ open ]                                   [ reveal ]│
└──────────────────────────────────────────────────────┘
```

The Settings tools card reads the new native count and the 32-tool
bound, and gives each row a lifecycle state — *active*, *trial (4/10
runs)*, *candidate*, *disabled* — with the candidate's evidence one
press away:

```
the tools                     19 native · 3 disabled · 32 bound
  deck            trial 4/10 runs · 0 calls · 1.2k declaration tokens
  vision          active · 6 calls · 83% success
  browser         candidate · 30 runs over 41 days · 0 calls
                  nearest active: web (0.71 description similarity)
                  [ disable ]
  computer_use    disabled · enable · 2 coverage misses since disabling
```

`fleet tools scan` prints the same columns as a table, and
`fleet tools advise` turns candidate rows into evidence-backed
`disable` suggestions. The card reads the last sweep beneath the rows:

```
last sweep 2026-10-24 03:00 · 2 disabled · 1 skipped (trial)
```

The Skills surface marks a disabled skill (`disabled · no use in 24
runs · enable`), the Cron card marks a disabled loop with its run
history, and the Router board marks a masked workload with the default
that serves it (`masked · standard_code serves · enable`). Every
disable names its evidence and its one-action reverse.

The same card carries the other direction, with the cluster's evidence
and the action one press away:

```
suggested
  loop      "refresh the corpus each morning" · 6 sessions · 08:10 ± 20m
            [ add the job ]  fleet cron add --name corpus-morning ...
  skill     read → edit → test → diff · 9 sessions · 78% of the cluster
            [ propose a skill ]
  workflow  test → tag → push · 4 sessions · crosses stages
            [ propose a workflow ]
  tool      117 rg/sed calls · [ propose a source change ]
```

## Increments

1. **Measure, then auto-disable tools (C0–C6, C21).** The
   `Declarations` record, the advisor's per-tool metrics (declared
   runs, successes, errors, tokens per call, declaration tokens, age,
   state), the trial window, the candidate gate, the sweep and its
   `lifecycle-sweep` job, the per-sweep cap, the calibration record,
   and the tools card's states. No tool is added and no code is removed
   in this increment; it is the baseline the rest are measured against.
2. **Skills, loops, and workflows on the same seam (C22–C26).** The
   `Surface` adapter, the `skills.disabled` filter with the `use_skill`
   refusal, role-named skills protected, the cron run history, the loop
   gate with expected fires, the router-workload mask and its fallback,
   the promotion pass and its per-kind suggestions, and the Skills,
   Cron, Router, and lifecycle-card surfaces.
3. **Lane capabilities (C7–C10).** `measure`, `imagine`, `browse`, the
   capability-to-lane mapping, lazy join, and the named-failure path.
   Reviewable with `fleet tools` and one live call each.
4. **Process control (C11).** The registry, `process`, incremental output,
   wait, kill, run-end cleanup and reporting.
5. **Code intelligence (C12–C13).** `apply_patch`, the patch parser and applier,
   `repo_map`, the declaration bound change to 32.
6. **Documents, fetch, artifacts (C14–C16).** `read_document`, the `web` `url`
   path, `send_file`, the artifact event and card.
7. **Recall and MCP (C17–C18).** `recall`; `resources/*` and `prompts/*` in the
   client; `mcp_resource` and `mcp_prompt`.
8. **Search and file ops (C19–C20).** `grep` context/type/multiline, `glob` via
   rg, `file_change` with its destructive class.

## Acceptance criteria

The lifecycle criteria (C0–C6, plus the sweep at C21) are increment 1;
the surface adapter, skills, loops, workflows, and promotion are
increment 2 (C22–C26); the capability criteria (C7–C20) follow; C27 is
the gate.

- [ ] **C0 — every run records what it declared.** One `Declarations`
  record (names, count, tier) is written before the first request; a
  replay reconstructs the list; a run with a disabled tool does not
  carry it. Pinned by an integration test that compares the first
  request's declarations with the log record.
- [ ] **C1 — the scan measures economics, and the archive bootstraps
  it.** `fleet tools scan` shows per tool: declared runs, calls,
  successes, errors, denials, tokens per call, declaration tokens,
  age, and state. A fixture with known calls, declarations, and sizes
  yields exact numbers; a tool with no declaration history is trial,
  not unused. `fleet tools coverage [--sessions N]` reports, per tool,
  the exact usage, the estimated declared runs from the introduction
  date and preconditions, and owner-intent coverage misses over the
  existing archive; a fixture with known owner turns yields the
  expected near-misses; the pass makes no generated text and labels
  every estimate. Disabled tools are included: a disabled tool whose
  owner turns clear the floor is a **reactivation** suggestion, with
  `fleet tools enable` as the action.
- [ ] **C2 — trial is a window.** A tool declared in fewer than
  `tools.trial_runs` runs, or younger than `tools.trial_days`, is
  marked trial and never appears as a disable candidate, whatever its
  calls. The fixture pins both boundaries.
- [ ] **C3 — the candidate gate holds.** A non-protected tool with
  zero successes over at least ten declared runs and fourteen days is
  a candidate; a tool with one success is not; a protected tool is
  flagged for review but not proposed; a tool with ten calls and an
  80% error rate is a "broken" candidate. Each boundary is a test.
- [ ] **C4 — disabling works and reverses.** `fleet tools disable
  <name>` persists the name, the next run does not declare it,
  `tool_search` does not return it, and a call to it answers "disabled"
  with the reason and the enable command rather than "unknown".
  `fleet tools enable <name>` puts it back, and the next run declares it
  again. A protected tool needs `--force`, and the refusal names the
  protection.
- [ ] **C5 — disabling is the only retirement.** No tool's
  implementation is removed by this plan; a disabled tool stays in
  `fleet tools` with its metrics and code, and the disable is a config
  entry, not a source change. Re-enabling it needs no rebuild and no
  re-install.
- [ ] **C6 — the card shows the lifecycle.** The Settings tools card
  shows each tool's state, trial progress, candidate evidence, and a
  and an enable action for disabled tools; a live capture refreshes
  `docs/images/settings-tools.png`.
- [ ] **C7 — the three lanes are reachable.** `measure`, `imagine`, and
  `browse` are declared on every ordinary run; a call lazy-joins only
  its lane; a run that never calls them spawns nothing; a missing key
  or browser is the lane's named reason, not a silent empty result.
  Pinned by a test that asserts the declaration names and a zero-spawn
  run.
- [ ] **C8 — `measure` mirrors the lane.** Its schema is the table +
  target + task-type shape; the result carries predictions, confidence,
  verdict, and the provider; the lane's `verdict=error` arrives as an
  `error` result.
- [ ] **C9 — `imagine` writes files.** The lane's data URIs are decoded
  and written inside the run folder (a default `generated/` directory or
  the requested destination); the result names each path, its size, the
  provider, and the model, and never carries base64 back to the model.
  The count and each image's bytes are bounded.
- [ ] **C10 — `browse` is one tool over three lane calls.** `open` and
  `read` in sequence see the same page; `screenshot` writes a PNG and
  returns its path; an unknown action is `error[validation]`.
- [ ] **C11 — background processes are controllable.** A background
  `bash` returns an id; `process output` returns only bytes since the
  last read; `process wait` honors its timeout and says "still running";
  `process kill` reaps and reports the exit; the registry is empty at
  run end and a surviving child is named in the report.
- [ ] **C12 — `apply_patch` is atomic per file.** A multi-file patch
  applies; a file whose hunk does not match is refused with the hunk
  and the nearest line, and no file is partially written; the test
  fixture covers add, update, delete, and a mismatch.
- [ ] **C13 — `repo_map` is budgeted and deterministic.** Two runs over
  the same tree produce the same text; the output stays within the
  token budget; definitions and reference counts are present; the
  result says it is heuristic.
- [ ] **C14 — documents are local-first.** `read_document` returns
  markdown with the parser named for a DOCX/XLSX/PDF fixture;
  `Escalate` is a named refusal that mentions the owner's Documents
  surface and performs no network call; an unsupported extension is
  refused by name.
- [ ] **C15 — a URL is read through the guard.** `web` with `url`
  returns the final URL, status, and text; a private-network URL is
  refused by the SSRF guard at the first hop and after a redirect;
  `web` without `url` is unchanged.
- [ ] **C16 — an artifact reaches the owner.** `send_file` emits one
  `Artifact` event and returns the path and size; a path outside the
  run folder is refused; the face renders the card with the name and
  size.
- [ ] **C17 — recall is read-only and bounded.** `recall` returns
  ranked facts and skills with scores and sources; a cold session
  index is a named "not warmed" answer; the call makes no model request
  (the mock request count is unchanged).
- [ ] **C18 — MCP resources and prompts are spoken.** `resources/list`
  and `prompts/list` follow cursors; `mcp_resource read` and
  `mcp_prompt get` return bounded content; a server without the
  capability names it; a fixture server drives the test.
- [ ] **C19 — search is guarded.** `grep` context, type, and multiline
  behave as declared (the fallback names what it cannot do); `glob`
  output honors `.gitignore` when rg is present and says when it fell
  back.
- [ ] **C20 — file operations are classified.** `file_change` moves,
  copies, deletes, and creates directories; `delete` and `move` ask in
  every mode; `.git`, the fleet home, and the session store are
  refused; a failed same-filesystem rename is a named copy+remove.
- [ ] **C21 — the sweep disables on its own.** `fleet tools sweep
  --dry-run` prints the candidates and changes nothing; `--apply` and
  the daily `lifecycle-sweep` job disable at most
  `tools.max_disable_per_sweep` gate candidates, skip protected, trial,
  and already-disabled tools, and record each disable in the
  calibration stream beside the config write. A config write that fails
  disables nothing and names the error.
- [ ] **C22 — one adapter, four surfaces.** The sweep, coverage, and
  card read a `Surface` adapter; tools, skills, and loops each
  implement declared/used/disable/enable; the workflow entry reads the
  router's workload chains from the calibration rows.
- [ ] **C23 — skills have the same lifecycle.** A skill's declared runs
  come from `Skills`, uses from `use_skill`; a gate candidate is
  removed from the prompt and refused by `use_skill` as disabled, while
  its file is untouched; `fleet skills enable` restores it; a
  role-named skill is protected; a fixture pins the metrics and the
  refusal.
- [ ] **C24 — loops have the same lifecycle.** The cron store records
  runs, successes, and last success; a job with at least three expected
  fires and no success over the window is disabled by the sweep
  (`enabled: false`), `lifecycle-sweep` and the corpus-refresh jobs are
  protected, and `fleet cron enable` restores it; a schedule-aware
  fixture pins the expected-fire gate.
- [ ] **C25 — workflows have the same lifecycle.** The router's
  workloads are measured from the calibration rows (selected versus
  routed); a workload below the idle rate after the window is masked, a
  verdict naming it falls back to the default and records `masked`, and
  `fleet workflows enable` restores it. The default, `vision`, and
  `composed` are protected; a fixture pins the mask, the fallback, and
  the restore. The execution topologies are named as the next unit and
  are not measured until the judgment plan reads them.
- [ ] **C26 — promotion is the mirror.** `fleet lifecycle promote`
  clusters the archive's session tasks, and for each cluster with
  enough members suggests the kind its evidence supports: a cadence
  cluster prints a `fleet cron add` line; a shared call sequence files
  a `skill` proposal; a stage-crossing sequence files a workflow
  proposal that names its lane or config change; shell work with no
  native tool files a `source` proposal. Nothing is auto-created; a
  fixture with repeated tasks, a repeated call sequence, and a cadence
  pins each suggestion, and a promoted artifact starts on trial.
- [ ] **C27 — the gate is green.** `cargo fmt --all --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo build
  --release`, `cargo test`; the new tools appear in `fleet tools` and
  the Settings count; the screenshot of the tools card is refreshed.

## Files to be touched

- `src/capabilities.rs` (new) — the capability-to-lane table, the
  argument mapping, the result naming
- `src/processes.rs` (new) — the background registry, output offsets,
  wait/kill
- `src/patch.rs` (new) — the patch grammar, the per-file atomic apply,
  the refusal with context
- `src/repo_map.rs` (new) — the definition patterns, the reference
  ranking, the token budget
- `src/tools.rs` — the new specs, `web`'s `url`, `grep`/`glob` changes,
  `file_change`, `MAX_VISIBLE_TOOLS`, the protected set
- `src/session.rs` — the `Declarations` record
- `src/lifecycle.rs` (new) — the `Surface` adapter, the sweep, the
  per-sweep cap, the calibration record, and the promotion pass
  (clustering, detectors, suggestions, dismissal)
- `src/improve.rs` — the proposal filing for a promoted tool, skill, or
  workflow (the existing `source` and `skill` targets; a loop prints the
  cron line instead)
- `src/cron.rs` — runs/successes/last success, and the default
  `lifecycle-sweep` job
- `src/skills.rs` — the disabled filter and the `use_skill` refusal
- `src/roles.rs` — read the skills a role names, for protection
- `src/route.rs` — the workload mask, the default fallback, and the
  `masked` reason on the decision and calibration row
- `src/calib.rs` — the masked-workload field the workflow scan reads
- `src/tool_advisor.rs` — declared runs, age, tokens per call,
  lifecycle states, the candidate gate, the archive backfill
  (introduction dates and preconditions); the `retire` advice kind
  renders as `disable` with a `fleet tools disable` action
- `src/tool_coverage.rs` (new) — the owner-intent coverage pass over
  the archive, the message-hash cache, the method/limits report
- `src/tools.rs` — the per-tool `introduced` table (backfill only)
- `src/settings.rs` — `tools.disabled`, `skills.disabled`,
  `workflows.disabled` and `workflows.protected`, the idle rate, the
  per-sweep cap, and the trial/disable thresholds, through the
  read-merge-write pattern
- `src/cli.rs`, `src/main.rs` — `fleet lifecycle sweep`, the per-kind
  `tools|skills|loops|workflows sweep`, `fleet lifecycle promote`,
  `fleet tools disable|enable`, `fleet skills disable|enable`,
  `fleet workflows disable|enable`, and `coverage`
- `src/agent.rs` — the capability `Work` variants, `process`, `recall`,
  `mcp_resource`/`mcp_prompt`, the artifact event, the registry on the
  agent, the declaration record, disabling from declarations and
  search
- `src/documents.rs` — expose what the tool needs (no behavior change)
- `src/web.rs` — no change; the tool calls `fetch`
- `src/mcp.rs` — `resources/*` and `prompts/*` with pagination; the
  base64 decoder becomes `pub(crate)` so `imagine` reuses it
- `src/gui/bridge.rs`, `src/gui/assets/app.js`, `app.css` — the
  artifact event and card, the tools count, and the lifecycle card's
  suggested section
- `tests/integration.rs`, `src/bin/mock-mcp.rs` — the fixtures
- docs: `docs/features/tools.md`, `docs/features/mcp.md`,
  `docs/architecture/tool-surface.md`, `CLAUDE.md`

## Out of scope

- **LSP and tree-sitter.** A real language server means a protocol
  client and a new dependency; `repo_map` and `apply_patch` are the
  slice that pays first.
- **Background delegation.** A child needs the approver and progress
  sink the parent holds for the whole run; making children
  asynchronous is an ownership redesign, its own plan.
- **`ask_user`, `exit_plan_mode`, owner notifications.** These are
  interaction surfaces, not tool availability; they need the approval
  channel's design.
- **PTY and stdin.** No portable PTY without a dependency; a pipe-only
  stdin hangs on prompts.
- **Speech and transcription.** The vendored `gen1` exposes only
  `gen1_imagine`; adding `gen1_speak`/`gen1_transcribe` is a lane
  change plus a re-vendor.
- **Notebook cell editing and PDF page ranges.** The documents
  extractor returns the whole document; page/sheet selection is a
  parser change.
- **Live vendor calibration of the new capabilities.** The measurement
  and generation lanes need keys; tests use the scripted server, and a
  live call is a review step, not a gate.
- **Deleting a disabled tool's code.** Disabling is the only action;
  the implementation and its tests stay, and a future need is one
  `enable` away. Deleting a tool's implementation is not a runtime
  action and is not planned here.
- **Usage verdicts before the trial window.** A tool younger than the
  trial is measured, not judged.
- **Auto-enable.** Reactivation is a suggestion with evidence; enabling
  automatically on every coverage miss would oscillate with the sweep.
  A re-enabled tool gets a cooldown before the sweep may disable it
  again.
- **Execution topologies until they are read.** The workload chains are
  covered; the `single`/`mixture`/`swarm` head is judged but discarded
  today, so it is not measured. It joins the workflow adapter when the
  judgment plan that reads it lands. Plan graphs are per-task
  declarations, not lifecycle units.
- **Editing the skill library.** Fleet writes only its own disabled
  list; a skill's file is never moved or deleted, because the canonical
  library is cross-harness.
- **Judging a loop on a quiet window.** A job is judged only after at
  least three expected fires, so a monthly job is not mistaken for a
  dead one.
- **Auto-creating a promoted artifact.** Promotion suggests and files;
  the owner applies through `improve` or runs the cron line. A bad
  suggestion never changes the surface on its own.
- **Inferring a pattern from the assistant's prose.** The pass reads
  session tasks, tool sequences, and timestamps; an assistant claiming
  "we do this every time" is not evidence.
- **Promoting without a cap.** Each run files at most
  `promote.max_suggestions` (default 5), and a dismissed cluster is
  remembered in `promote.dismissed` so the card does not nag.
- **Disabling on a coverage score.** Coverage ranks owner turns for
  review; it never proposes a disable. The gate reads exact usage
  and the estimate, not similarity.
- **Inferring need from the model's own prose.** The coverage pass
  reads owner turns and the log's tool events, not an assistant's
  claim that it lacked a capability.
- **Embedding the whole archive by default.** The pass is opt-in and
  bounded; the scan stays cheap.

## Verification

- C7–C10, C12, C14, C17, C18: unit tests in the new modules plus the
  scripted MCP server; the capability mapping is a pure table tested
  without a lane.
- C11, C20: integration tests with a real child process and a scratch
  tree.
- C13: a fixture tree with known definitions and a budget assertion.
- C15: the SSRF tests the GUI fetcher already has, reused through the
  tool; one private-address case and one redirect case.
- C16, C27: the event asserted in the bridge test; the face verified
  live with a refreshed screenshot.
- C19: the existing search tests extended; `rg`-present and
  `rg`-absent paths.
- C0–C6, C21: a fixture corridor with a never-called tool, a trial
  tool, a protected tool, a broken tool, and a disabled tool; exact
  scan numbers; the sweep's dry run and apply with the cap; the CLI
  write and the next run's declarations.
- C22–C25: the surface adapter fixture; the skills disabled/refusal
  test with the file untouched; the cron expected-fire and run-history
  fixture; the workload mask/fallback fixture over scripted calibration
  rows.
- C26: a fixture with three repeated tasks at a cadence, a repeated
  call sequence, and a shell-only pattern; the expected suggestions and
  cron line; nothing created; the cap and the dismissal.
- The live pass: once the window accumulates, `fleet tools advise` and
  `fleet lifecycle sweep --dry-run` over this machine's sessions; the
  first reading is a baseline, not a verdict.

## Linked artifacts

- `docs/architecture/tool-surface.md` — the audit and the evidence.
- `docs/features/tools.md` — the surface as it stands, including the
  live calibration.
- `docs/plans/2026-10-23-tool-surface-and-lane-resolution.md` — the
  plan this one builds on.
- `src/documents.rs`, `src/web.rs` — the parsers and the guard that
  already exist and only need a tool seam.

## Risks

- **Capability wrappers can lie about availability.** A wrapper is
  declared even when the lane has no key or no Chrome. Mitigation: the
  first call is the probe, and its failure is the lane's own named
  reason; no wrapper answers with an empty success.
- **Ten new declarations cross the bound.** Mitigation: the bound moves
  to 32, the MCP half stays deferred behind `tool_search`, and the
  tools card shows the count. If 32 proves too broad, the first
  candidates to defer are `repo_map` and `file_change` behind the
  search.
- **`apply_patch` parsing is a new failure surface.** Mitigation: the
  patch is parsed fully before any write, each file is checked
  against its text, and the refusal names the hunk and the nearest
  line; the atomic write path is the same one `edit` uses.
- **The process registry can leak children.** Mitigation: it is per
  run, closed at run end, and a surviving child is named rather than
  killed; a killed child is reaped.
- **Documents can contain credentials.** Mitigation: the tool respects
  the run folder and the module's refusal codes; it performs no
  egress, and `Escalate` is a refusal rather than a quiet upload.
- **MCP resources can be large.** Mitigation: every list paginates,
  every read is bounded, and a server that does not implement the
  capability is named.
- **The GUI artifact card is new surface.** Mitigation: the event is
  typed and tested, the card is one `boardCard`-style block, and the
  path is validated before emission.
- **Auto-disabling a rare-but-decisive tool.** Mitigation: the idle
  rate, the protected set, the trial window, a cap of three per sweep,
  a dry run, the calibration row, and a one-command `enable`; a
  re-enabled tool is cooldown-protected for `tools.reactivate_days`.
- **A biased window.** Mitigation: every row names its window, run
  count, and age; a tool with no declaration history is trial.
- **Coverage similarity is noisy.** Mitigation: the calibrated floor
  and margin, the sys1 class as a second cut, and review-only use; a
  near-duplicate tool is named rather than counted as a miss.
- **The introduction table rots.** Mitigation: it is backfill-only,
  the exact `Declarations` record replaces it after the first window,
  and a test asserts every current tool has an entry.
- **Disabled tools referenced elsewhere.** Mitigation: a disabled call
  resolves to a named `disabled` result, not `unknown`; aliases still
  map.
- **A disabled tool going stale.** Mitigation: the coverage pass watches
  disabled tools too and suggests `enable` when the need returns.
- **The sweep racing a run.** Mitigation: the sweep runs from cron, not
  inside a session; a run's declarations are fixed when it starts, so a
  disable applies to the next run; the config write uses the existing
  read-merge-write and the home lock.
- **A skill disabled in fleet but needed elsewhere.** Mitigation: the
  canonical skill file is never touched; the disabled list is fleet's
  own view.
- **A loop judged on a quiet schedule.** Mitigation: expected fires, the
  three-fire floor, and protected maintenance jobs.
- **The sweep disabling its own scheduler or the corpus refresh.**
  Mitigation: `lifecycle-sweep` and the corpus-refresh jobs are in the
  loop-protected set.
- **A promotion that is a coincidence.** The same task can recur for
  different reasons. Mitigation: the cluster threshold, the minimum
  member count, the evidence and example on every row, the
  `promote.max_suggestions` cap, dismissal, and the owner's gate; a
  promoted artifact starts on trial, so the disable lifecycle can
  remove it if it does not take.
- **Promotion growing the surface it just pruned.** Mitigation: the two
  drives share the trial window and the owner's gate; a new tool or
  skill is proposed, never injected, and the idle rate applies to it
  like everything else.
- **Masking the only workload that could serve a request.** Mitigation:
  the default, `vision`, and `composed` are protected; the idle rate,
  trial window, cap, and dry run apply; a mask is client-side and
  reversible with `fleet workflows enable`. The route-decision cache is
  read first, then the mask is applied, so a cached verdict cannot slip
  a disabled workload through. A masked workload records `masked` and
  the default that served.
