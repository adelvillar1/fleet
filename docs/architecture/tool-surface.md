# The tool surface

An analysis of what the model can do in this harness, what the public
harnesses have converged on, and what to build in what order.

Status: analysis, 2026-10-23. Not a plan; the build order at the end is
input for one.

## Verdict

Fleet has a 2026 harness wrapped around a 2024 tool interface. Routing,
lanes, memory, delegation, calibration, checkpoints and a browser face
are all ahead of what most harnesses ship. The thing the model actually
touches — four built-ins, one whole-file write, no search, no ranged
read, no progress ledger — is smaller than SWE-agent's 2024 interface,
and every one of those tools is a special case in a 280-line branch
chain.

The gap is not "fleet needs thirty tools." The public evidence says
the opposite: tool *count* stops paying around 30–50 and turns negative
when the catalog is unfiltered. The gap is three things, in order:

1. **The action space is too lossy.** A whole-file write and an
   unbounded read make every edit a rewrite and every large file a
   context event. There is no search primitive at all.
2. **The channel is closed and stringly typed.** Arguments arrive as a
   raw JSON string, results leave as one `String` plus a bool, errors
   are prose prefixes, and adding a tool touches four places. Nothing
   can be improved cheaply, so nothing is.
3. **Nothing measures the tool layer.** `src/eval.rs` has 21 scenarios
   and not one of them grades a tool call's arguments, cost, or
   recoverability. Every claim in this document about fleet is a
   code-reading; every claim about improvement would be too.

The visible symptoms — no `grep`, no `edit` — are worth fixing, but
fixing only them leaves the ceiling in place.

## 1. The surface as it stands

### 1.1 What the model sees

Fifteen statically declared tools plus whatever MCP lanes joined.

| Group | Tools | Declared when | Cite |
|---|---|---|---|
| Closed four | `bash`, `read_file`, `write_file`, `list_dir` | always | `src/tools.rs:1150-1159` |
| Beside the four | `use_skill`, `remember`, `supersede`, `improve`, `convention`, `plan`, `deck`, `vision`, `web`, `computer_use`, `subagent` | by run shape | `src/agent.rs:1317-1400` |
| Lanes | `mcp__<server>__<tool>` | on join | `src/tools.rs:1026-1147` |

The four built-ins carry one-line descriptions and four JSON Schemas
(`src/tools.rs:1177-1213`). `read_file` takes `path` and nothing else.
`write_file` takes `path` and `content` and replaces the file. `bash`
takes `command` and nothing else — no timeout, no background, no
working directory. `list_dir` takes `path`, is non-recursive, and caps
display at 1,000 entries.

That is the whole of the model's ability to look at and change a
repository through structured tools. Everything else goes through
`bash`.

### 1.2 How a call travels

`prepare_call` (`src/agent.rs:2516-2800`) is an if/else chain over tool
name strings: file-access refusal, approval, plan-mode refusal, scope
refusal, then one branch per tool, each re-parsing the raw argument
string (`serde_json::from_str`, `src/agent.rs:2588`). Execution needs a
second enum (`Work`, `src/agent.rs:3111-3198`) and a matching arm in
`run_work` (`3202-3500`). A new tool touches the constant, the
preparer, `Work`, and `run_work` — plus any new policy branch.

The consequences are structural, not cosmetic:

- **No schema validation.** The declared `parameters` are sent to the
  provider and never enforced locally (`src/api.rs:312-357`; no
  validation call exists). Every tool re-validates by hand, in its own
  vocabulary.
- **No structured results.** `ToolOutcome { output: String, truncated:
  bool }` (`src/tools.rs:1287-1292`) is the whole result type. Images,
  attachments, exit classes and artifact paths are special-cased
  (`vision`, `deck`, `computer_use`) or lost.
- **Errors are prose.** `error:`, `denied:`, `subagent failed (...)`,
  and guard prose. The model string-matches; the harness cannot count
  error classes.
- **Unknown tools get no help.** `error: unknown tool \`x\`` with no
  list and no nearest match (`src/agent.rs:2786`). Same for
  `use_skill` (`src/skills.rs:715`) and lanes
  (`src/agent.rs:4855`).

### 1.3 Output handling

One bound, 64 KiB, applied *after* the full output is in memory:
`src/tools.rs:17`, `1303-1320`. `bash` accumulates every drained byte
before bounding (`1475-1527`); `read_file` reads the whole file
(`1642-1648`); `list_dir` collects every entry; `vision` reads the
whole image before checking its 8 MiB cap (`src/agent.rs:947-954`).

Three tools escape the bound entirely: `vision` returns the routed
model's text unbounded (`src/agent.rs:3284-3329`), `subagent` appends
delta blocks after the cap (`src/subagents.rs:2121-2132`), and MCP
answers may be 8 MiB per line before the agent bounds them.

Truncation is a byte count with a marker and no continuation. A
truncated `read_file` cannot be resumed because there is no offset to
resume from. A `bash` result that spills has no artifact path, so the
model's only recovery is to re-run the command.

### 1.4 The loop around the tools

The loop is better than the interface. Batched calls are planned into
waves with writer conflicts deferred (`src/agent.rs:2346-2401`,
`4020-4066`), results are recorded in batch order (`2403-2450`), and
concurrency is capped at 3 (`src/agent.rs:133`).

What is missing is the parts that make tools feel fast or safe:

- **No cancellation.** The control file is read between model calls
  only (`1710-1734`); `run_bounded` has no stop channel
  (`4080-4127`). Stop during a long `bash` waits for the 120 s cap.
- **No background execution.** Every command is foreground. A dev
  server or a slow suite is a 120-second failure, and the model's only
  workaround is `&`, which loses the output.
- **No per-turn call cap.** A model returning 100 calls runs all 100 in
  waves of 3.
- **No retry policy and no recovery hint.** The first failure is the
  expensive one: SWE-agent measured first-attempt edit success at 90.5%
  and 57.2% after one failure, with cascading failed edits behind 23.4%
  of unresolved trajectories.

### 1.5 The MCP client

Fleet speaks `2024-11-05` (`src/mcp.rs:40,446`) over stdio, with
`initialize`, `tools/list` (one page, `{}`, `465-473`) and `tools/call`.

Supported: text content, `isError`, lazily joined vendored lanes, a dead
lane that fails fast, timeouts per method (`42-58`).

Not supported: resources, prompts, sampling, elicitation, structured
content or `outputSchema`, tool annotations, progress, cancellation,
pagination, `list_changed`, server-initiated requests, and non-text
content (reduced to `[image]`, `src/mcp.rs:269-285`). `inputSchema` is
copied verbatim and never validated (`638-661`).

That is enough to call a tool and not enough to integrate a modern
server. It is also, by itself, a reason a user's existing MCP servers
underperform in fleet compared with the harness that discovers them.

### 1.6 A concrete failure, end to end

Take the ordinary task "change one condition in a 4,000-line file."

1. `read_file` returns the whole file, then the 64 KiB bound cuts it.
   The model receives the first ~64 KiB with a byte-count marker and no
   line numbers. It has no offset to continue from, because the schema
   has no offset.
2. If the cut lands before the target, `bash` is the only way to see
   the rest — `sed -n`, which is exactly the interface SWE-agent's
   ablations showed scoring 7.7 points below a structured edit tool.
3. To change one line, `write_file` requires the entire file content as
   an argument. The model must reproduce 4,000 lines exactly, from a
   truncated view, in one JSON string.
4. Nothing checks the result. There is no uniqueness rule, no
   read-before-write hash, no diff back. If the reproduction dropped a
   brace, the model learns it from a later compiler error, one
   round-trip on, at full conversation cost.

The token cost is the visible part. The expensive part is that step 3
is a silent-corruption machine: the model is asked to be a lossless
copy of a file it could not fully see. This is the single path through
which most edits in this harness happen today.

### 1.7 What is genuinely good

Worth saying before the criticism:

- `convention` — asking the tree what it already calls a concept, with
  sites, is rarer than it should be and directly attacks the
  invented-synonym failure.
- `remember` / `supersede` — a memory with basis, shapes, phrases and
  missing-companions is more disciplined than most.
- `subagent` — "never silence", a typed outcome, and delta claims
  verified against the child log (`src/subagents.rs:1884-1902`) are
  stronger than the `Task` tools it competes with.
- The session log records exactly what the model received, including
  truncation and approval, which is the foundation every measurement
  recommendation below needs.

The rest of this document is about closing the gap between that
quality and the four tools underneath it.

## 2. What the public harnesses converged on

Researched 2026-10-23 against primary docs and source.

| Harness | Core tools | Mechanism worth stealing |
|---|---|---|
| Claude Code | ~45 built-ins, MCP deferred | `Read` with line numbers and 25k-token cap; `Edit` with a uniqueness rule; `Bash` timeout + background + spill-to-file; older tool outputs cleared before summarizing |
| Codex CLI | ~15 + agent namespace | `exec_command` with `yield_time_ms` and `max_output_tokens`; freeform `apply_patch`; `update_plan`; `tool_search` over deferred tools (BM25) |
| Gemini CLI | ~20 | `read_file(start_line,end_line)` mandated; `grep_search` "preferred over shell grep"; `write_todos`; `complete_task`; per-model schema sets |
| Cline / Roo | 9 | `editor` XOR `apply_patch` selected by model family; read-before-edit; `read_file` indentation mode; one-tool-per-turn XML fallback |
| OpenHands | `terminal`, `file_editor`, `task_tracker` | `undo_edit`; status aliases for todo items; per-model presets (`gpt5`, `gemini`) |
| Aider | repo map + edit formats | tree-sitter → dependency graph → PageRank → token budget; edit format matched to model family |
| Factory | ~17 | `ApplyPatch` auto-added for OpenAI models alongside `Edit`; risk-tiered autonomy |
| Amp | ~3 documented | Mode and tool definitions frozen per thread for cache stability |

The convergence is real: by 2026 the mainstream vocabulary is about
fifteen names — read, glob, grep, edit, write, apply_patch, bash, web,
todo, task, skill, question — and harnesses differ mainly in which
subset they expose and to which model.

### 2.1 The numbers that should drive decisions

**Editing.** SWE-agent's ablations (SWE-bench Lite, GPT-4 Turbo, base
18.0): remove the edit tool → 10.3; edit without a lint gate → 15.0;
whole-file view instead of a 100-line window → 12.7; 30-line window →
14.3; iterative search instead of capped search → 12.0; no search →
15.7; full history instead of collapsing old observations → 15.0; raw
shell baseline → 11.0. Interface choices moved the result by as much
as 7.7 points — the same order as a model upgrade.

**Edit anchoring.** *Look Before You Leap* (arXiv 2609.11957) measured
line-number-anchored edits corrupting 99.1% of files under a one-line
shift, while content-anchored search/replace failed cleanly, and an
anchor-and-verify applier had one silent misapplication in 8,320
trials (0.01%). Content-anchored is the safer default; line numbers are
a reading aid, not an edit coordinate.

**Output budget.** Returning a retrieved section instead of inlining a
tool output cost GPT-5.1/5.2 11.3 and 7.9 accuracy points; naming the
authoritative output file recovered 6.3–7.9 (arXiv 2610.26836). The
lesson is not "truncate more" but "always name where the whole thing
lives."

**Old results.** Claude Code clears older tool outputs before
summarizing. The 2026 harness-design study (arXiv 2609.20804) found
rule-based elision beat LLM summarization, and a rolling drop of tool
responses measured +1.7 points at −27.7% cost (arXiv 2610.28577).

**Catalog size.** Anthropic measures selection degrading past 30–50
tools, and 5 MCP servers at ~55k tokens of definitions; deferred
loading cut an 85% token share and moved Opus 4 from 49% to 74% on
their selection eval. OSWorld-MCP's unfiltered 158-tool exposure
dropped success from 20.5% to 15.5%.

**Descriptions.** An audit of 856 MCP tools found 97.1% with at least
one description smell and 56% failing to state purpose clearly;
rewriting raised median task success 5.85 points (arXiv 2602.14878) —
at the cost of 67% more steps.

**Silent failure.** RITE (arXiv 2610.27777) found silent tool errors —
plausible but wrong values — roughly halve task success, worse than
loud failures.

**Long-horizon.** MAST (arXiv 2503.13657) across 1,642 traces: step
repetition 15.7%, unaware termination 12.4%, premature termination
6.2%, framework failure rates 41–86.7%. A progress ledger is cheap
insurance against exactly those.

**Bash-only is viable.** mini-swe-agent claims >74% on SWE-bench
Verified with ~100 lines and no structured tools, and the 2026 harness
study found predefined tools help bash-weak models while bash-strong
models do as well, cheaper, with bash alone. That is the honest
argument for fixing `bash` first and adding structure for guardrails,
not out of faith.

### 2.2 Evidence strength

Strong, ablated or multi-study: windowed observation, edit guardrails,
explicit error feedback, description/retrieval quality, old-observation
elision, content-anchored edits, context rot, pass^k revealing
instability, traces missing calls without a tool-side ledger.

Vendor eval, large effect but not peer-reviewed: deferred tool loading
and tool search (Anthropic), programmatic tool calling (−37% tokens),
code execution with MCP (−98.7% tokens), 25k-token caps.

Moderate: todo/planning gains (no clean "todo tool vs none" RCT), fewer
namespaced tools, parallel calls' accuracy effect (mostly latency and
cost).

## 3. The recommended target surface

Not thirty tools. Eight built-ins, a real channel, and a measurement
loop. Each of the eleven existing special tools stays.

### 3.1 The eight

| Tool | Arguments | Semantics that matter |
|---|---|---|
| `bash` | `command`, `timeout_ms?`, `run_in_background?`, `description?` | default 120 s, clamp to 600 s; on timeout, promote to background rather than kill silently; head+tail excerpt on failure; spill full output to an artifact path and name it; report exit code always |
| `read_file` | `path`, `offset?`, `limit?` | line-numbered; header `path (N lines)`; window default ~2,000 lines; `(showing 1–200 of 3,841; continue with offset=201)`; empty-file and past-EOF notices; binary detection by NUL sniff with a named error |
| `write_file` | `path`, `content` | atomic temp+rename; size cap; returns bytes and line count |
| `edit` | `path`, `old_string`, `new_string`, `replace_all?` | requires a prior read of that path this run; `old_string` must be unique unless `replace_all`; zero matches returns the nearest plausible line, not just "not found"; multiple matches returns count and line numbers; writes atomically; returns a capped unified diff |
| `list_dir` | `path`, `depth?`, `pattern?` | depth default 1; sizes; cap 1,000 with an explicit count |
| `grep` | `pattern`, `path?`, `glob?`, `case_sensitive?`, `max?` | default cap 50 with "narrow the query"; output modes files/content/count; honors `.gitignore` inside a repo; never an iterative next/prev stream — that ablation scored *worse than no search* |
| `glob` | `pattern`, `path?` | sorted by mtime, cap 100, truncation flag |
| `todo` | `items[{step,status}]` | full-list replacement; `pending/in_progress/completed` plus aliases (`todo`, `done`); no dependencies, no ids; re-injected after compaction |

Why `edit` rather than a patch tool first: content-anchored replacement
is the format with the strongest measured failure profile, and it is
what Claude Code, Gemini CLI, OpenHands, OpenCode and Roo all use. Add
`apply_patch` later, and only as a per-model preset for model families
that are trained on it — Factory auto-adds it for OpenAI models beside
`Edit`, and Cline treats the two as mutually exclusive by model.

Why `grep`/`glob` despite bash: they are the guardrails. A tool can cap
at 50, sort, mark truncation, and report a match count; `bash grep`
cannot, and its output is subject to the same unbounded capture and
post-hoc 64 KiB cut. Shelling out to `rg` when present, falling back to
`grep -rn`, keeps the implementation at zero new crates.

### 3.2 Descriptions are part of the interface

The four current descriptions are one sentence each: "Read a UTF-8 text
file. Fails for binary files; use bash for those." That is enough for a
human and thin for a model choosing between tools. The measured effect
is large: an audit of 856 MCP tool descriptions found 97.1% with at
least one smell, and rewriting them raised median task success by 5.85
points.

The target shape, for `edit` as an example:

```
Replace one exact passage in a file you have already read.

Use this for every change to an existing file. Do not use write_file to
change part of a file: it replaces the whole file and loses anything you
did not reproduce exactly. Do not use bash sed/awk for edits.

old_string must match the file byte for byte, including indentation, and
must appear exactly once. Include two or three surrounding lines so that
it is unique; do not use line numbers, which shift. If the call fails
with "not found", re-read the file — it changed since your last read. If
it fails with "appears 3 times", extend old_string with more context or
pass replace_all.

Returns the changed line range and a diff of the change.
```

Four properties to copy into every tool description: what it is for,
what to use *instead* when it does not fit, the one failure mode that
costs a round trip and how to recover, and what comes back. One
concrete example per tool where the arguments are non-obvious.

### 3.3 The channel

One trait, one registry, one result type.

```rust
trait Tool {
    fn name(&self) -> &'static str;
    fn spec(&self, env: &ToolEnv) -> ToolSpec;   // declared when present
    fn policy(&self) -> Policy;                  // read-only | writes | shell | network | always-ask
    fn execute<'a>(&'a self, cx: &'a Ctx, args: Value) -> BoxFuture<'a, ToolResult>;
}

struct ToolResult {
    text: String,                 // what the model sees
    structured: Option<Value>,    // what the log and UI get
    truncated: Option<Truncation>,// omitted bytes, artifact path
    artifacts: Vec<Artifact>,     // files, images, diffs
    error: Option<ToolError>,     // class + hint
}
```

Four rules that follow:

1. **Validate before execute.** A small JSON-Schema subset — type,
   required, enum, pattern, min/max — covers every schema fleet
   declares. Unknown arguments are a warning in the result, not a
   failure; providers add fields.
2. **Errors carry a class and a hint.** `validation`, `not_found`,
   `denied`, `timeout`, `conflict`, `internal`, `lane_dead`. The model
   sees `error[validation]: ... ; hint: ...`. The harness counts
   classes.
3. **Unknown names list alternatives.** Tool, skill and lane errors
   name the nearest valid options. `use_skill` already has the full
   list in the prompt; the error should too.
4. **Track reads.** A per-run set of paths read (with a content hash)
   backs the read-before-edit rule and gives the model a truthful "this
   file changed since you read it" instead of a silent overwrite.

This is the expensive part and it is what makes every later change
cheap. It should not be sold as a refactor; it is the difference
between a tool set that can improve and one that cannot.

### 3.4 Context policy for tool output

Three changes to `src/compaction.rs` and the result seam:

- **Elide before summarizing.** Replace tool results older than the
  last N (6 is a reasonable start) with a one-line stub: tool name, byte
  count, artifact path, first line. Then summarize what remains.
  Evidence: faster and better than summary-first, and it preserves
  recoverability.
- **Spill, never drop.** `bash` and oversized reads write full output
  to `<FLEET_HOME>/sessions/<id>/calls/<n>.out`; the stub names the
  path. Recovery is `read_file` on the artifact, which is a tool the
  model already knows.
- **Re-inject the todo ledger** after a fold. It is the one piece of
  state whose loss produces step repetition.

### 3.5 MCP

Two stages.

**Stage one — the missing half of 2024-11-05:**

- Read `annotations` and use `readOnlyHint` / `destructiveHint` to pick
  an approval tier. Never trust them for policy (the spec says so); an
  explicit per-server allow/deny still wins.
- Persist `outputSchema` and validate `structuredContent`; keep text as
  the fallback.
- Follow `nextCursor` in `tools/list`.
- Map non-text content: images become artifacts with a path, embedded
  resources become artifacts, `[image]` disappears.
- Send `notifications/cancelled` on timeout; refresh on
  `tools/list_changed`.
- Make a pre-join failure visible in the session, not only the progress
  log (`src/agent.rs:1690-1693`).
- Validate ingested schemas and drop tools whose root is not an object.
  Normalize descriptions at ingest.

**Stage two — the 2026-07-28 protocol:** stateless requests with
per-request `_meta`, `server/discover` with fallback to `initialize`,
`resultType` and MRTR handling, `subscriptions/listen`. Either build
it dual-era by hand or adopt `rmcp`, the official Rust SDK, for the
transport and types while keeping fleet's registry and policy. Hand-
rolling a second protocol era is the kind of work a dependency exists
to absorb.

**Catalog discipline** regardless of stage: deterministic declaration
order, defer server tools beyond ~30 visible, and add a `tool_search`
meta-tool when the catalog crosses that line. Namespacing already
follows the spec's recommendation.

## 4. Build order

Sequenced so each step is verifiable and the next one is cheaper.

### P0 — Correctness and instrumentation

Fix what is wrong and build the ledger that grades everything after.

- `vision` absolute-path escape: `root.join(asked)` replaces the root
  when `asked` is absolute (`src/agent.rs:923-946`). Canonicalize,
  refuse outside-root, sniff the media type instead of defaulting to
  PNG.
- Plan mode is not read-only: it refuses `write_file` and
  `computer_use` only, while `bash`, `remember`, `supersede`, `improve`
  and `plan` all write (`src/agent.rs:1091-1103`). Decide and enforce.
- `computer_use` promises a screenshot "rides to you as an image"
  (`src/tools.rs:700-701`) and returns only a path
  (`src/agent.rs:3805-3816`). Attach the image or fix the sentence.
- TUI passes `lane_specs: None` (`src/tui/mod.rs:842-849`), so user
  MCP servers are unreachable there.
- `supersede` dispatches on `starts_with` (`src/agent.rs:2680`).
- `write_file` is a non-atomic `fs::write` (`src/tools.rs:1693`).
- `Gate::approve` returns `approves_without_asking`, so "ask" is
  unrepresentable and headless modes deny where the comments say they
  prompt (`src/approvals.rs:262-274,325-333`). Make the approver
  tri-state or state the denial contract.
- Instrument: per call record name, raw args, validated args,
  duration, result bytes, truncated bytes, error class, artifact path.
  Three layers — emitted, validated, executed — because that is how a
  model error is told apart from a harness error.

Acceptance: every existing test green; a session log that answers "what
did the model ask for, what ran, and what came back" for every call.

### P1 — The four that change the ceiling

`read_file` window with line numbers, `edit`, `grep`, `glob`. Plus
`bash` timeout and background.

Add them inside the current dispatch if P2 has not landed; they are
worth having before the refactor. But do them with the result shape
`ToolResult` in mind so P2 is a move, not a rewrite.

Acceptance: a 4,000-line file can be read, edited at a known location,
and the edit verified without a whole-file rewrite; a repo-wide search
returns a capped, counted result; a 10-minute build can be started
without occupying the turn.

### P2 — The channel

Registry, schema validation, error classes, artifacts, read tracking,
cancellation token, per-turn call cap.

Acceptance: adding a tool touches one module; a malformed argument
returns a named validation error naming the field; `Stop` during
`bash` returns within ~1s.

### P3 — Context and MCP stage one

Tool-result elision with spill artifacts, todo ledger and re-injection,
MCP annotations, pagination, structured output, images, cancellation.

Acceptance: a 30-call session compacts without losing any artifact
path; a fold preserves the ledger.

### P4 — Measurement, then the rest

- `fleet tools audit`: read a session corpus, report schema-valid
  rate, arg-error rate by class, p95 result bytes, truncation share,
  redundant-call rate, recovery rate after an error, calls per
  completed task.
- **Replay A/B.** Replay a recorded trajectory's tool calls through a
  new implementation in a scratch worktree and diff results. For
  read-only tools this is exact; for writes, compare the resulting
  files. This is the cheapest honest test of "did the new edit tool
  help," and it uses the log fleet already writes.
- BFCL subset as the external check: selection, arguments,
  irrelevance/abstention, parallel-call validity. Cheapest high-signal
  public benchmark; API cost only.
- Per-model presets: `apply_patch` for families trained on it; a
  bash-lean preset for models that do well there.
- A `consult` tool — route one question to another lane or model under
  a budget. This is Claude Code's advisor and Codex's agent namespace
  collapsed to what fleet already has: routing and lanes. It is the one
  place the tool layer can leapfrog rather than catch up.

## 5. What not to build

- **A memory MCP server.** Fleet has its own memory with basis and
  shapes; running a second store creates two sources of truth and
  context bloat.
- **`task-master`, `sequential-thinking`, or any 30-tool default
  catalog.** 36 tools is ~21k tokens by that server's own README.
- **A semantic search index before grep/glob exist.** Cursor's measured
  gains from embeddings are largest on 1,000+ file repos and modest
  otherwise; the cheap primitive comes first.
- **A code-execution meta-tool before a sandbox.** The token savings
  are real and large, but fleet's `bash` is already unsandboxed, so a
  second unsandboxed executor adds risk without adding a boundary.
- **A tool-count race.** Fifteen good tools with guardrails beat forty
  with smells; the evidence on the 30–50 threshold is consistent.

## 6. Open questions the owner should settle

1. **Is `bash` meant to be the sandbox?** Fleet has no sandbox and says
   so (`src/tools.rs:1356-1360`). If it stays that way, structured
   tools are about guardrails and measurement, not safety, and the
   approval model is the only boundary.
2. **How much per-model divergence is acceptable?** A preset system is
   the clear 2026 answer, but it multiplies the test matrix. A single
   `edit` everywhere is simpler and defensible for models that accept
   it.
3. **Does the TUI stay a first-class surface?** If not, leaving it
   unable to reach MCP is a documentation task; if so it is a P0.
4. **Do lanes get resources and prompts, or stay tools-only?** The
   protocol work in stage two is substantially larger if resources and
   prompts are in scope.

## Sources

Fleet code: `src/tools.rs`, `src/agent.rs`, `src/mcp.rs`,
`src/approvals.rs`, `src/compaction.rs`, `src/subagents.rs`,
`src/eval.rs`, `src/tui/mod.rs`.

Interface evidence: SWE-agent (arXiv 2405.15793); CodeAct
(2402.01030); *Look Before You Leap* (2609.11957); Diff-XYZ
(2510.12487); *An Empirical Study of Harness Design for Coding Agents*
(2609.20804); *Same Simulation, Different Evidence* (2610.26836);
*Cache Engineering for Agents* (2610.28577); MCP tool descriptions
(2602.14878); RITE silent errors (2610.27777); MAST (2503.13657);
Lost in the Middle (2307.03172); RULER (2404.06654); NoLiMa
(2502.05167); Agentless (2407.01489); LLMCompiler (2312.04511);
OSWorld-MCP (2510.24563); BFCL leaderboard and harness.

Harness docs, read 2026-10-23: Claude Code tools/permissions/hooks/MCP
reference and changelog v2.1.309; `openai/codex` at `810501d`; Gemini
CLI at `a713410`; Cline and Roo Code tool definitions; OpenHands
software-agent-sdk presets; Aider repo-map and edit-format docs;
Factory subagents and autonomy docs; Amp tools docs; OpenCode and
OpenHands tool docs; MCP specification 2026-07-28; Anthropic
engineering on writing tools, advanced tool use, code execution with
MCP, and multi-agent research.
