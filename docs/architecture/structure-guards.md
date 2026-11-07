# The structure guards

The cross-cutting invariants this repo enforces with text-scan tests,
named bounds, and list pins — the guards that fail the suite rather
than the compile, and that nothing at edit time surfaces. Every one
here was paid for by a real session losing a compile cycle to
discovering it (the 2026-11-07 dream-plan build met all four classes
in one pass). `scripts/edit-preflight.sh` names the ones an edit will
meet, in seconds, before the first `cargo` call; this doc is the one
home for what they are and what to do when one fires.

## A. The socket guards — the lane-asking beats

Three beats spend a lane ask (sys1 judgment calls): `judge`,
`delegate`, `explain`. Each is guarded by a text-scan test that reads
a list of synchronous-path modules with `include_str!` and refuses
any reference to the beat:

- `src/judge/tests.rs` → `no_synchronous_path_references_the_judgment_beat`
  (scans memory.rs, agent.rs, tools.rs, session.rs, calib.rs, route.rs,
  compaction.rs, cron.rs, improve.rs, boards.rs, skills.rs,
  subagents.rs, and the tui modules for `judge::`, `crate::judge`,
  `JUDGE_BUDGET_ENV`, `JUDGE_ESTIMATED_ASK_USD`)
- `src/delegate/tests.rs` → `no_synchronous_path_references_the_delegation_beat`
  (27 modules — wider: api.rs, mcp.rs, lanes.rs, home.rs, install.rs,
  net.rs, cli.rs are in its set on purpose)
- `src/explain/tests.rs` → `no_synchronous_path_references_the_explanation_beat`
  (its own list, same shape)

**Trigger:** any new reference to `judge::` / `delegate::` /
`explain::` (or a beat's named constant) from a scanned module.

**The intent, not just the rule:** a beat that spends a lane ask must
be unreachable from a path that runs on every turn. The sanctioned
shape when scheduled or CLI work needs a beat: the lane-owning wrapper
lives in its own module beside the pass that calls it, and only the
scheduled thread, the CLI verb, and the face's runner reference it —
the seams that already own scheduled lane work (`src/dream.rs` is the
precedent; `memory.rs` runs the pass lane-free with the judge
injected).

**Beside the guards:** a beat's calib row type is constructed in
exactly two modules — `calib` (which defines it) and the beat (which
writes it). `route.rs` keeps its own JSON-literal helper in its tests
for exactly this reason; do not "fix" that duplication.

## B. The tool-surface bounds

`src/agent.rs`: `MAX_VISIBLE_TOOLS` (36) and `MAX_DEFINITION_TOKENS`
(10,050). The tokens bound pins the total size of the tool
declarations the native surface sends, through
`the_native_surface_fits_its_own_bounds`, and drives three runtime
withholding walks in the surface curator — when the surface outgrows
the bound, the curator withholds tools and declares `tool_search`
instead.

**Trigger:** any edit to a tool's schema or description text
(`src/tools.rs`), any new tool.

**What to do when it fires:** the protocol is written on the constant
itself — measure the real count, then either trim the prose or move
the constant with the crossing named in its history (it records all
three). Never move it silently. Because the bound also feeds the
runtime walks, a move changes what withholds: the declared-list test
`a_run_with_memory_declares_the_write_path_and_one_without_it_does_not`
pins the whole list, and its expectation follows the walk (at 10,000
the `remember` schema's `source` field withheld `apply_patch` and
declared `tool_search`; at 10,050 neither happens).

## C. The calibration row discipline

Adding a `Row` variant in `src/calib.rs` touches, all in the same
change: the variant, the `op()` arm, the `ts()` arm, an `OP_*`
constant, and the `OPS` array — whose expected list is itself pinned
by a test in calib.rs's own module. Read-side grouping follows `OPS`,
so the face and `fleet calib` pick the row up from the array alone.

## D. The request-path / cold-build split

`src/gui/related.rs` → `a_request_path_never_builds_the_cold_corpus`:
the request-path `corpus()` must read the cache (`build_corpus_cached`)
and never call the cold builder (`build_corpus`) or a live embedder
(`probed_embedder`); the `warm()` path must still build. The compiler
enforces half of it (corpus takes no embedder); the text scan pins the
rest.

**Trigger:** touching the corpus build paths in `related.rs`.

## E. The struct-field fan-out

Not a guard test — the compiler finds it — but the cost is a compile
cycle, and the literals live in *other* modules. Adding a field to a
shared corpus struct means hand-built initializers break at every one
of these sites:

- `memory::Fact`, `memory::Remember`, `memory::FactEdit` — tests in
  `memory.rs`, plus constructors in `onboard.rs`,
  `session_recall.rs::fact_for`, `gui/bridge.rs` (the board writer),
  `tui/app.rs` (via `boards::MemoryFact`)
- `boards::MemoryFact` — `boards.rs::read_memory` and the snapshot
  fixture in `tui/app.rs`
- `cli::MemoriesOptions`, `cli::RememberOptions` — the parse tests'
  expected literals in `cli.rs`
- the calib row structs — the row helpers in `calib.rs`'s tests

New fields must land with every constructor in the same commit; the
preflight names the list.

## F. The face harness's fixture counts

`tests/face/harness.mjs` asserts exact rendered counts against its
fake documents ("N more facts", revealed-row totals). Adding a fixture
fact to `memoryDoc` (or any sibling document) shifts every count
assertion that reads it. The assertions live beside their checks;
search for the count strings when the fixture changes.

## The loop that uses this

`scripts/edit-preflight.sh [paths...]` — defaults to the working
tree's changes, names the guards the edit will meet, pure grep, no
compile. Run it before the first `cargo` call of a build session; the
house build-loop skill carries the habit (pins before edits, scoped
test runs while iterating, one full gate at the end, a progress line
between compile cycles).
