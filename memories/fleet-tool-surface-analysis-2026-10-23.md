---
name: fleet-tool-surface-analysis-2026-10-23
description: "2026-10-23 deep analysis then BUILD of fleet's tool surface, FINISHED the same day — eight built-ins, validation + aliases + read-before-edit, tiering at 28 tools/8k tokens, tool_search (sys1 class + sem1 cosine, FLOOR 0.68 CALIBRATED on live sem1), the ToolResolution ledger with `fleet tools audit`, elision/spill/todo-ledger, bash promotion + in-flight stop, MCP pages/structured/images/annotations, the two-provider grep dispatch, and the advisor; plan `completed`, all C0–C16 addressed, gate 1,588"
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

Owner asked 2026-10-23 for a deep analysis of how to improve the harness,
"especially tools," then — after reading it and adding three mid-turn
directives — for the plan and its implementation ("wow me"; later `/goal
finish the plan`). Documents: the analysis
`docs/architecture/tool-surface.md`, the contract
`docs/plans/2026-10-23-tool-surface-and-lane-resolution.md` (**status:
completed**, with a Progress section), behaviour `docs/features/tools.md`.

**Commits, in order** (all gate-green, plain `cargo test`):
`05915c0` tools + lanes, `75ce5d7` advisor, `cf6f68b` ledger/audit/limits,
`a050f4c` bash promotion, `6719168` criteria tests, `6210626` stop +
registry evidence, `bebc827` MCP image artifacts, `ca8e974` annotations +
dispatch + face + screenshot, `d69b109` live floor calibration. Final gate:
fmt, clippy `-D warnings`, release build, **1,588 tests** across 11 suites.

**The core finding, unchanged.** A 2026 harness over a 2024 tool
interface. Tool *count* was never the lever (selection degrades past 30–50;
OSWorld-MCP 20.5% → 15.5% exposing all 158). The levers were content-anchored
edits (line-anchored corrupts 99.1% under a one-line shift), windowed
line-numbered reads, capped search (iterative paging scored *worse than no
search*), a todo ledger (MAST step repetition 15.7%), and eliding old results
before summarizing. Evidence base:
[[harness-tool-surface-research-2026-10]].

**What shipped (the whole arc).**

- **Eight built-ins** — `bash`, `read_file` (lines, offset/limit, named
  empty/past-end/binary notices), `write_file` (atomic), `list_dir` (depth),
  `edit` (content-anchored, prior-read required, names lines on ambiguity and
  the closest line on zero), `grep` (files/content/count, cap + count),
  `glob`, `todo`. Modules: `src/fileops.rs`, `src/search.rs`,
  `src/tool_router.rs`, `src/tool_advisor.rs`, `src/tool_audit.rs`.
- **Resolution** — exact → foreign alias (`str_replace`, `apply_diff`, …) →
  nearest-name hint → the tool list; schema validation before execution
  (`error[validation]: field …`); unknown arguments dropped with a note.
- **Tiering** — native core always declared; MCP tools folded while the
  request fits 28 tools / 8,000 definition tokens; past that the core stays
  byte-stable and `tool_search` is declared.
- **`tool_search`** — sys1's new `tool_route` task names a closed capability
  class (incl. `none`), sem1 ranks cards by cosine with cached vectors, then
  a floor + margin decide; lexical fallback has its OWN floor; no model in
  the path. **Calibrated live 2026-10-23** (see the constants below).
- **Ledger + audit** — one `ToolResolution` record per call (requested,
  resolved, exact/alias/retrieved/refused, reason, catalog, tier);
  `fleet tools audit [<session>]` prints validity, error classes, p95 bytes,
  truncation, redundancy, recovery, methods.
- **Limits** — 40 calls/turn (the rest refused by name), spill of a cut
  result to `<session>/calls/*.out`, fold elision (last 6 results verbatim)
  plus todo-ledger re-injection (pinned through a real forced fold), and the
  route verdict reordering native tools by profile.
- **bash** — `timeout_ms`, `run_in_background`, **promotion** instead of a
  kill (a blown deadline leaves the command running and names pid + stdout
  + stderr files), and **in-flight stop**: a watcher reads the control file
  every 200 ms and the bash poll kills the child, so `/stop` does not wait.
- **MCP** — cursor pagination, `structuredContent` fallback,
  `notifications/cancelled` on timeout, image blocks written to a file and
  named (small base64 decoder, no new crate), pre-join failure recorded as
  `LaneFailed`, and **`annotations` read**: a `destructiveHint` forces the
  prompt path in `approvals::Gate` and can never grant.
- **Dispatch wired** — `grep` has two providers (`rg`, `grep`) and the
  warm→healthy→order policy picks one; the result names the fallback reason.
- **Face** — the live and replayed tool card show the resolution method; the
  Settings board carries a *tools* card (8 native, tier bounds, last
  session's audit), captured live at `docs/images/settings-tools.png`
  (headless Chrome hung on the event stream — the capture had to go through
  CDP and scroll to the card).

**The retrieval constants are measured, not guessed** (2026-10-23, live
embeddinggemma-2 on 127.0.0.1:8901, `scripts/tool-floor-calibration.py`
reproduces them): realistic tool catalog — true intents **0.691–0.802**,
no-tool requests **0.602–0.668**, near-duplicate read tools **0.009–0.010**
apart; the plan's 40-tool corpus 24/24 top-1 at 0.832–0.853 with its neat
tokens. So `FLOOR = 0.68` (16/16 true pass, 6/6 no-tool abstain), `MARGIN =
0.02` (near-dupes ambiguous, 0.024+ decisive), and `LEXICAL_FLOOR = 0.30`
because token-overlap scores are on another scale. **The original 0.35 was
never a threshold against this embedder — unrelated text scores ~0.60.**
Limitation the data showed: without the sys1 class, "find every place TODO
appears" ranks the *todo* tool first; the class text is what fixes it.

**Evidence that matters most:** the advisor's first live run over 30 sessions
— `write_file` **fails 18 of 22 calls (82%)**, `remember` 10 of 18, 73 grep
+ 36 edit + 8 glob calls went through bash, 13 enabled MCP servers never
called. The 82% is the top open defect (not yet diagnosed). The advisor also
had to be fixed: it first counted special tools (`remember`, `supersede`,
`convention`) as unknown and printed 35 noisy rows — check
`tools::known_tool_names()` before calling a name unknown, and collapse
retire rows into one line.

**The vendored caveat.** sys1's `tool_route` task lives in
`services/sys1/src/sys1/tasks.py`; the upstream core needs the same task or
a re-vendor silently drops tool_search's first stage.

**Still open:** `Gate::approve` returns a bool, so "ask" is unrepresentable
headless (the gate's hint set only adds denials). The `write_file` 82% is
undiagnosed. The follow-up availability plan
([[fleet-tool-availability-gaps-2026-10-23]]) is drafted and awaiting
approval — it is the piece that makes `sdm1`/`gen1`/`browser` reachable.

**The criteria were re-audited twice, and each pass found what a green gate
cannot.** First audit: C12's dispatch had no two-provider capability, C15's
annotations were unread, C13 and C16's screenshot unsettled. Second: the
verifier caught that C10's floor was tuned on a fixture embedder — running the
corpus through live sem1 moved `FLOOR` 0.35 → **0.68** and split out
`LEXICAL_FLOOR = 0.30` ([[a-fixture-corpus-flatters-a-retriever]]). A green
suite covers the assertions that exist; a criterion phrased as "measured" is
not satisfied by a fixture measuring it.

**How to apply:** the declaration-count trap
([[adding-a-tool-breaks-the-declaration-counts]]) is now an 8-tool world —
pinned lists live in `agent.rs`, `tools.rs`, `subagents.rs` (scope, 12
admissible) and `tests/integration.rs`; adding a `ToolResolution` sibling
record also shifts every positional `records[3]` assertion (use a
`first_result` helper). When asked to make a tool call "not a guess", reach
for a lane-owned **decision** (sys1 class) plus **geometry** (sem1), never a
model call repairing a guess — and **calibrate the geometric threshold
against the live embedder before trusting it** (see
[[embeddings-cannot-label-a-session-project]]: the same embedder's cosines
are soft, ~0.6–0.8 for everything, which is why the floor has to be high).
Related: [[mcp-manager-build-state-and-coordination]],
[[fleet-judgment-beats-spawn-per-row]],
[[fleet-skills-invocation-is-model-only]].
