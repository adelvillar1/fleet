# Deep dive: the router and the self-healing surface — 2026-11-15

Two verified reads, one question: **what does the harness already have
that a housekeeper could stand on, and what does it only appear to
have?** Every claim below carries a reference. Where a doc and the code
disagree, the code wins and the disagreement is named — that is the
point of the exercise.

Read for this dive: `route.rs`, `route_cache.rs`, `judge.rs`,
`delegate.rs`, `generate.rs`, `lanes.rs` (roster legs); `health.rs`,
`lifecycle.rs`, `cron.rs`, `dream.rs`, `memory.rs` (consolidation),
`checkpoints.rs`, `processes.rs`, `identity.rs`, `workspace.rs`,
`anomalies.rs`, `demand.rs`, `improve.rs`, `journal.rs`, `lesson.rs`,
`verify.rs`, and the two existing reviews
([self-healing](2026-11-15-self-healing-review.md),
[memory dreaming](deep-dive-2026-11-07-memory-dreaming.md)).

Headline: **the housekeeper is mostly a wiring problem.** The meters,
the proposal store, the gate, the inbox and a durable task ledger all
exist. Almost none of them are scheduled, and the ones that are have no
principal. One thing genuinely does not exist: a routing chain for a
small model.

---

# Part 1 — The router

## The shape, in one line

Roster (8 providers) → judgment (`sys1`/`route`, seven closed labels,
15 s) → walk (`Leader` / `Judged` / `Named`) → order built and the
DeepSeek reserve appended (`with_tail`, `route.rs:2499`) → demotion
reorder → stream with per-class failover → one `RouteRow`
(`calib.rs:64-105`), written by `calib.rs`, never by `route.rs`.

## The table as compiled

| workload | chain (target → fallbacks) | tail |
|---|---|---|
| `quick` | stepfun/step-3.7-flash → mimo-v2.6-flash → GLM-5.3-Flash | deepseek |
| `standard_code` | step-3.7-flash → GLM-5.3-Flash → mimo-v2.6-flash | deepseek |
| `frontend` | **deepseek/deepseek-flash** → GLM-5.3-Flash → ollama-cloud/glm-5.3-flash | **none** |
| `hard` | step-5-preview → mimo-v2.6-pro → GLM-5.3-Flash | deepseek |
| `prose` | step-3.7-flash → GLM-5.3-Flash → qwen3.8-flash | deepseek |
| `deep_context` | step-5-preview → GLM-5.3-Flash → mimo-v2.6-pro | deepseek |
| `vision` | GLM-5.3-Flash → mimo-v2.6-flash → step-5-preview → gemma4:31b | deepseek |
| `composed` | blend1/blend1 → step-3.7-flash → GLM-5.3-Flash → mimo-v2.6-flash | deepseek |

## Findings

**R-1 — No workload names a small model.** Every chain walks
flash-or-above: `step-3.7-flash`, `GLM-5.3-Flash`, `mimo-v2.6-flash`,
`step-5-preview`, `qwen3.8-flash`, `deepseek-flash`. There is no 1–2B in
the table, so the only small model fleet calls is the one in
`generate.rs` — which the router never sees (R-2).

**R-2 — The prose leg is outside routing entirely.** `generate.rs:30`
holds `TEXT_MODEL = "lfm3-1.7b"` as a constant, passed literally into
blend1's tool (`generate.rs:116-120`). The module imports `crate::lanes`
and `crate::mcp` and nothing else — no `route`, no cache, no pin, no
`RouteDecision`, no row. So the leg that writes folds, titles and
archive summaries has: no failover, no spend accounting, no served-model
record in the calibration stream, and no resource contract beyond a 120 s
call budget.

**R-3 — The declared roster cannot serve what the leg calls.** `liquid`
declares exactly one model, `lfm2.5-24b-a2b`, 131,072
(`route.rs:534-550`). `lfm3-1.7b` is not in it. It works today only
because `generate` bypasses validation; `resolve_model_pin` validates the
*provider* and non-empty model, not membership in the declared list. A
`housekeeping` workload naming `lfm3-1.7b` therefore requires a roster
edit, not just a table row.

**R-4 — One failure policy, for every caller.** `Router::chat_stream`
has no caller-mode: cron (`cron.rs`), delegated children and the leader
all walk the same order, with 600 s per candidate, no retry, no backoff,
and no `Retry-After` handling. A background job inherits the interactive
rule — fail over until someone answers — even though nobody is waiting.

**R-5 — No cost ceiling on the routing path.** There is no USD cap, no
spend ledger and no `cost_usd` on `RouteRow` anywhere in `route.rs` or
`calib.rs`. Caps exist only on four batch beats, all reserved-per-ask:
`judge` $0.05/day, `delegate` $0.05/day, `desk` $0.05/day, `explain`
$0.50/day. The only guard on routing is `gates.allow_api_models`, which
is a hosting gate, not a number.

**R-6 — `frontend` breaks the reserve ruling.** `with_tail` returns the
order *unchanged* if `deepseek` appears anywhere in it
(`route.rs:2504-2506`). `frontend` names deepseek first
(`route.rs:638-653`), so its order ends on `ollama-cloud/glm-5.3-flash`,
not on the reserve. The comment above the table still says "no workload
targets deepseek" (`route.rs:616-619`). Two doc/code contradictions in
one place, and a real behavioural divergence from the owner's
2026-10-21 ruling.

**R-7 — Demotion is invisible to measurement.** Trigger: three
consecutive failed attempts for a provider within three day-files
(`calib.rs:2613`, `2667`). Effect: reorders candidates to the end and
keeps the selected one at the head (`route.rs:4330-4346`) — it never
changes the selection. Recording: a clause inside `RouteRow.reason`
(`route.rs:4161-4166`). No demotion row, so no reader can compute which
providers are demoted, for how long, or whether the demotion was right.

**R-8 — The route cache is keyed on a constant.** The cache key literally
ends `walk=hard` (`route_cache.rs:154-165`) and does not vary with the
actual walk or workload. There is no TTL and no config-invalidation, so
editing `routing.json` does not invalidate a cached judgment; only the
lane's launch identity does. `rekey` clears `entries` and `curate` but
**not** `validate` (`route_cache.rs:189-194`), against its own doc
comment. A malformed cache reads as empty with no log.

**R-9 — A dead comment about a keyless machine.** KI-044's comment at
`route.rs:2702-2708` says a keyless machine skips the lane spawn; no such
check exists — after a cache miss the code goes straight to
`McpServer::spawn`.

**R-10 — No modality flags, and vision is a name.** No `Candidate`,
`ModelSettings` or `ModelOption` carries a text/image/video field; the
only modality claim in the tree is a comment about GLM. The `vision`
workload is chosen by name (`carries_image` → the vision chain,
`route.rs:2092-2136`) and its chain is not filtered by capability, so a
text-only model at the tail would receive the image.

**R-11 — One side-route does not route.** `workload_decision` builds an
order from the *compiled* defaults only — ignoring `routing.json`, the
home and the mask (`route.rs:1939-1957`) — and its caller constructs a
routed client then calls `Client::chat`, which does not consult the
router (`api.rs:745-769`; only `chat_stream` does). As written, the
vision tool's side-route never traverses `Router::chat_stream`.

---

# Part 2 — The self-healing surface

## Four kinds, by trigger

| kind | mechanisms |
|---|---|
| **timer-bound** | health probe loop (30 s, face only); cron tick (1 s + startup catch-up); homekeeping (3600 s: consolidation + dream); checkpoint reclaim (900 s); eval cadence (daily 04:00 + model change) |
| **request-path** | provider failover/quota unlock/demotion; route-cache discard; embedder degradation + retries; torn session tail; unreadable fact skip; disk reserve admission/overrun; ENOSPC sweep; run-end child shutdown |
| **manual verb** | `install`, `reload`, `recover`, `hygiene`, `tools sweep`, `memories --consolidate/--validate`, `skills --curate`, `improve apply/undo`, `mcp health`, `calib --anomalies`, `demand --cluster`, `eval --gaps`, `pair`, `doctor` |
| **dead path** | `journal::curtail`; `lesson::probe_loop`/`teach`; `verify::FactGate` |

Callability separates cleanly along that line: the timer-bound and manual
mechanisms are `pub` functions with a small surface (a `home`/root path
plus a `calib::Stream`), so another component *can* invoke them on
demand. The request-path ones cannot be invoked — they are steps inside
`Agent::run`, `Router::chat_stream` or memory load. That distinction is
the plan's main constraint: **a housekeeper can trigger the former and
can only observe the latter.**

## Findings

**H-1 — The health loop runs only in the face.** `start_health_loop` has
exactly one call site, `gui/mod.rs:331`. Chat and TUI spawn homekeeping,
reclaim and cron, and never health. So in the two surfaces where
interactive work happens, nothing probes the substrate, the keys, the
lanes, the MCP servers or the release feed. Silence is an absent spawn.

**H-2 — Four maintenance threads are unsupervised.** The cron tick, the
homekeeping loop, the reclaim loop and the health task have no outer
catch and no supervisor; a panic ends that thread for the process's life.
Cron catches panics per fire, which is not the same thing.

**H-3 — Most healing is ungraded because it records nothing.** Of the
mechanisms surveyed, three produce a record something later reads: route
rows (demotion reads them — R-7), cron's job counters
(`lifecycle::loop_candidates` reads them), and `improve`'s gate evidence.
Everything else writes stderr or a card. A heal with no record cannot be
scored, so it cannot be tuned, and it cannot be told from a heal that
never ran.

**H-4 — Repair results are discarded at the call site.** `let _ =
layerfs_step` (`gui/bridge.rs:803-806`), `let _ =
record_server_probe` (`gui/bridge.rs:700`), `let _ = bootstrap_lane`
(`gui/bridge.rs:4177-4180`). A failed heal is indistinguishable from a
successful one until something else notices.

**H-5 — Several failures are invisible outside the face.** A
configured-but-broken MCP server is probed only on the face's ~10-minute
cadence or a CLI verb; outside the face, `enabled_specs` never probes. A
lane's health outside the face is a `stat()` on `mcp_server.py`, so a
venv that cannot serve reads healthy. A present-but-invalid API key is
only caught by the slow-tier `GET /models`, which only the face runs.

**H-6 — The manual floor is real, but the review's example is stale.**
`scripts/daily-maintenance.sh` runs from an external LaunchAgent at 09:20
and carries the cache sweep, tmp retirement, corpus commit,
`fleet hygiene`, the plan-worktree sweep, lane builds and the footprint
reading — none of it invoked from `src/`. **But** `cache-sweep.sh` *is*
invoked from `src/` on ENOSPC (`health.rs:596-605`), so the review's
"not invoked from `src/`" is out of date. The correction matters: the
clearance stage would put the sweep on a schedule, not bring it into
existence.

**H-7 — Three built surfaces have no runtime caller.** `journal::curtail`
exists and is never called, so the documented 30-day bound never
truncates (`journal.rs:204`; test-only). `lesson::probe_loop`/`teach`
exist and are never called, so the learning loop is not wired into chat,
TUI or face. `verify::FactGate` exists and is never called, so fact
verification is a library. Three capabilities the harness believes it
has.

**H-8 — Background work is anonymous.** Cron jobs pass `principal:
None` (`cron.rs:1571`), delegated children pass `None`
(`delegate.rs:354`), subagent children inherit the parent's. `HealthReport`,
`Consolidation`, `DreamReport` and the prune functions carry no
principal at all even as a field. Only `workspace::Task` has an
assignee (`workspace.rs:174`).

**H-9 — A latched permission already exists.** `cron::Job` carries
`auto_approve: bool` (`cron.rs:656`), and a job without it has its calls
skipped (`cron.rs:613`). So "no latched approver on unattended work" is
a ruling about an existing mechanism, not a rule to introduce — and the
default is already safe.

---

# Part 3 — What the housekeeper was going to build, that exists

This is what changes the plan. Set beside the idle-cycle doc, four of its
five subsystems are already present and unconnected.

| idle-cycle need | already in the tree | state |
|---|---|---|
| **Meter** | `anomalies::pass` — six deterministic classes (`fallback-walk`, `tool-failure-loop`, `lanes-down`, `delegation-maxed`, `run-failure-cluster`, `cache-collapse`) | CLI only (`main.rs:2754`), no timer, **records nothing** |
| | `lifecycle::sweep` — unused tools, skills, loops, workflows | daily 02:30 cron job; records `lifecycle` rows |
| | `demand::collect` — families, rates, trends from calib | CLI; `--cluster` files proposals |
| | `footprint` — tree RSS/process ceilings, 30-day standing free | run-time and CLI |
| | `health::probe` — five lines, tiered | face only (H-1) |
| **Recommend → gate** | `demand --cluster`: clusters bad classes ≥5 and files `improve` proposals with acceptance criteria and target shas (`demand.rs:617-683`) | the recommender, in embryo, for source changes |
| **Approve → apply** | `improve`: proposal → fmt/clippy/build/test → eval contrast → revert or commit → lesson | the APPROVED class, built |
| **Ask** | `inbox.json`, five live classes (`seat`, `onboard`, `ask`, `cron`, `doctor`) + dismissed/archived decisions | the ask surface, live-derived |
| **Ledger** | `workspace::Task { id, title, status: working|blocked|done, assignee: PrincipalId, binding }` + edges in `graph.jsonl` | the recommendation record's natural home, principal already in the shape |

The two gaps that remain genuinely new are the ones the last three
sessions named: **a routing chain for a small model** (R-1..R-3) and **a
principal on background work** (H-8). Everything else is scheduling,
typing and connecting.

---

# Part 4 — What this changes in the idle-cycle doc

Six corrections, one of them structural.

1. **The trigger has a prerequisite.** "Idle rides the health loop's slow
   tier" is wrong as written: the health loop is face-only (H-1). Either
   the idle cycle spawns its own probe, or `health` is hoisted out of the
   face. This is a P0 prerequisite, not a detail.
2. **Clearance is a schedule, not an introduction.** The cache sweep
   already runs on ENOSPC (H-6). The stage's honest claim is that the
   sweep stops depending on a failure to happen.
3. **The prose chain needs a roster edit, not a table row.** `lfm3-1.7b`
   is undeclared (R-3), and nothing validates a model against the
   declared list.
4. **The meter is four existing readers, one of which has no timer.**
   The doc should name `anomalies`, `lifecycle`, `demand` and `footprint`
   instead of describing eight new readers. `anomalies` is the interesting
   one: it already computes the classes and throws the result away.
5. **The approved class has an executor.** `improve` already gates,
   applies, reverts and records. The doc should not imply a second apply
   path.
6. **"No latched approver" is a ruling on `cron.auto_approve`** (H-9),
   already defaulting safe — it should be stated against that mechanism,
   not as a new principle.

---

# Part 5 — Findings for the KI ledger (rule 10)

Twelve defect-shaped findings this read produced. Each is independent of
the idle cycle and worth landing on its own.

| # | issue | evidence | next action |
|---|---|---|---|
| 1 | `frontend`'s order never ends on the reserve, and the table's comment contradicts it | `route.rs:638-653`, `2504-2506`, `616-619` | decide whether the ruling means "last" or "present"; fix clause or comment |
| 2 | Route cache keys on a constant `walk=hard`, has no TTL, ignores config edits, and `rekey` skips `validate` | `route_cache.rs:154-165`, `189-194` | key on the walked workload; add config-hash invalidation; clear all namespaces |
| 3 | KI-044's keyless-spawn precondition is a comment with no code | `route.rs:2702-2708`, `2742` | add the check or retire the comment |
| 4 | `generate.rs` is outside the router: no chain, no cap, no row, no validation | `generate.rs:30`, `116-120` | bind to a workload (Part 1 R-2) |
| 5 | `liquid` does not declare `lfm3-1.7b` though `generate` calls it | `route.rs:534-550` | declare it; consider validating pins against declared models |
| 6 | The health loop runs only in the face | `gui/mod.rs:331` | hoist the probe out of the face |
| 7 | Four maintenance threads have no supervisor | `cron.rs`, `memory.rs`, `checkpoints.rs`, `gui/bridge.rs` | one catch-and-restart supervisor per loop, named |
| 8 | Repair results are discarded at three call sites | `gui/bridge.rs:700`, `803-806`, `4177-4180` | record each result, even as a line |
| 9 | `journal::curtail` is never called, so the documented bound never runs | `journal.rs:204` | schedule it; the journal is unbounded today |
| 10 | `lesson`'s learning loop and `verify`'s fact gate have no caller | `lesson.rs`, `verify.rs` | wire or retire; dead capability reads as available |
| 11 | Demotion is recorded only as prose inside `reason` | `route.rs:4161-4166` | a demotion row, so it can be graded |
| 12 | The vision side-route never traverses the router | `route.rs:1939-1957`, `api.rs:745-769` | route it, or delete the indirection |

---

## The line this dive draws

The harness has already built the parts of self-healing that *detect and
shut down*, and it does that well. What it has not built is the part that
**scores, schedules and attributes**: nothing grades a heal, four loops
run unsupervised, and background work has no principal. That is a
narrower and more honest target for the next slice than "a housekeeper" —
three schedulers, one typed record, one principal, and the four meters
that already exist.
