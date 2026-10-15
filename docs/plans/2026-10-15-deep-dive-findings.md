# The deep dive's findings, addressed

Status: active — implementation began 2026-10-15; C19 resolved by the
owner's "implement plan as designed" with the plan's recommended
default (the bump and the row move to after the first successful model
response)
Date: 2026-10-15

## Why

The 2026-10-15 deep dive ([docs/architecture/deep-dive-2026-10-15.md](../architecture/deep-dive-2026-10-15.md))
classified its findings as confirmed defects, assumed gaps, and enhancements.
This plan addresses all of them. Per the deep-dive discipline, nothing here
treats the owner's design intent as a defect: the two items that are
genuinely the owner's ruling carry a [NEEDS CLARIFICATION] marker rather
than an invented default, and the plan stays draft until they are resolved.

The findings, restated so each phase below has a named target:

1. **Confirmed** — the TUI settings pane cannot reach the list-shaped
   surfaces: `commit_setting` carries the four prefixed branches
   (`src/tui/app.rs:701-716`) but the editor is only ever seeded from
   `settings::states`, the ten-entry scalar table
   (`src/settings.rs:395-433`), so no pane-reachable key takes them. The
   settings-surface plan promised roster candidates and workload
   preferences in the pane; the gap was never disclosed.
2. **Confirmed** — doc drift, five instances: the calib module header
   names fewer ops than the OPS array carries; the consolidate prose
   names fewer actions than ACTIONS carries; the eval probe docs name
   fewer scenarios than the corpus holds; the graph unlock prose names
   fewer conditions than `is_unlocked` requires; the services README
   names four lane cores where five resolve.
3. **Confirmed** — the delegation beat's skip cells have no live
   consumer (only their own tests and the eval scenario), and the two
   cell implementations disagree on whether waived rows count toward
   the fit sample: `src/delegate.rs:1608-1610` counts them (with a
   documented rationale — a waived row is still a row the lane judged,
   so it counts toward the sample's size while staying out of the
   rate), while `src/calib.rs:1956` excludes them from the graded
   count outright. Two documented rules for one question is a
   divergence; one of them is wrong for the ride-back.
4. **Assumed gap** — KI-068: recall bumps and `recall_hit` rows fire at
   assembly time, before the model call, so a failed run still bumps
   every surfaced fact. The loop's numbers describe recall attempts,
   not recalls the model saw.
5. **Assumed gap** — the graph's unlock prose versus its code (folded
   into phase 3's doc drift; the code's stricter reading is the safe
   one).
6. **Assumed gap** — session recall's own feedback loop (the archive
   plan's C7), deferred by the module's own record because a synthetic
   fact's path is the session log.
7. **Enhancement** — the settings pane's list surfaces: the close of
   finding 1, including the settings plan's unbuilt C4 (the in-pane
   confirmation).
8. **Enhancement** — `fleet doctor` names the web leg: the web-tool
   plan's deferred C6.

**The surface-parity audit** (the settings surfaces, per the
multi-surface rule — two renderings over one writer):

| Surface | Scalar knobs | Provider list | Workload list | Destructive confirmation |
|---|---|---|---|---|
| `fleet settings set` (CLI) | works | works | works | `force` flag |
| TUI settings pane (`ctrl+,`) | works | unreachable | unreachable | none |

The plan's job on phase 1 is to make the second row match the first
without letting the two surfaces drift.

## Acceptance criteria

### Phase 1 — the settings pane reaches the list surfaces

- **C1** The pane renders the provider roster and the workload
  preference table as cards beside the scalar knobs, never a plain-text
  table (verified: a TestBackend render assertion on the Settings mode;
  the TUI test suite green).
- **C2** Picking a provider card opens the editor seeded with that
  entry's current value; committing routes through the writer's own
  `set_provider` / `remove_provider`, never a pane-local path
  (verified: a state-machine test drives open → pick → edit → commit
  against a scratch home).
- **C3** Removing an override of a compiled-in provider reverts it from
  the pane; removing one the file itself added refuses and names the
  rule — the same rule, read from the same compiled-in roster, the CLI
  enforces (verified: both paths tested through the state machine).
- **C4** The destructive class asks in the pane before it lands — the
  settings plan's unbuilt C4: a confirmation step naming exactly what
  will be removed, `y` confirms, every other key abandons with nothing
  written (verified: a state-machine test on both outcomes).
- **C5** `workload.set` / `workload.remove` are reachable the same way,
  with the writer's own validation naming a bad candidate (verified).
- **C6** The CLI's behavior is byte-for-byte unchanged — no `cli.rs`
  edit, the existing settings tests green (verified: the gate).
- **C7** `provider.add` remains the JSON-buffer shape with the writer's
  field validation naming a misspelled field; the structured-fields
  editor is named as a follow-up in Out of scope, not silently dropped.

### Phase 2 — doctor names the web leg

- **C8** `fleet doctor` carries a web-leg line: keyed (naming the env
  var, never a value) with blend1 resolving, or the leg's own fail-open
  naming the missing key — informational, never a failing check, the
  way the roster's unkeyed candidate is named without failing
  (verified: live `fleet doctor` output; a hermetic evaluator unit
  test in `doctor.rs`'s existing pattern).
- **C9** `fleet lanes`' blend1 card renders the web leg's availability
  from blend1's own doctor report — the parse keeps the key the report
  already carries instead of dropping it as an unrendered extra
  (verified: the parse test; live `fleet lanes`).
- **C10** The doctor check resolves the key through the same precedence
  a run uses (env → `<FLEET_HOME>/.env`), spawns nothing, and opens no
  socket — the evaluator takes gathered facts like every other doctor
  check (verified: hermetic in tests).

### Phase 3 — the doc drift

- **C11** The calib module header names the ops the OPS array actually
  carries, and the consolidate prose names the actions ACTIONS carries
  (verified: the header's count equals `OPS.len()`; a grep of the prose
  against the constant).
- **C12** The eval probe docs name the corpus's real scenario count
  (verified: the count read from `corpus()`).
- **C13** The graph unlock prose names the three booleans
  `is_unlocked` requires (verified: read against the code).
- **C14** The services README names five lane cores (verified: the
  lanes that resolve in `fleet doctor`).
- **C15** No behavior changes in this phase (verified: the gate green
  with no test delta).

### Phase 4 — the delegation cells

- **C16** The two cell implementations agree on the fit arithmetic:
  waived rows are excluded from the fit count in both, so a cell never
  earns its ride-back on rows the owner set aside — the conservative
  direction for a decision that moves work. The delegate side's
  documented rationale is addressed rather than ignored: its point was
  that six insufficient rows are six of the eight, so `needs_rows`'s
  *display* keeps counting every graded row (right, wrong,
  insufficient, waived) as what the day holds, while the fit *decision*
  excludes the waived. Both cards keep showing the waived count as
  set-aside (verified: tests pinned on both sides; the delegate cell's
  existing count assertions move with the change and are re-based
  deliberately, not wholesale).
- **C17** The cells gain their named consumer: `fleet delegate` renders
  what the calibration would move for a brief shape beside the graded
  rows — the ride-back made visible — and the delegation-loop-closes
  eval scenario asserts it (verified: the scenario; a live dry-run).
- **C18** The rejected alternative is recorded in the plan: building a
  full provider-suggestion surface, turned down because the child runs
  on the parent's route under the same-model rule, so a
  provider-picking surface has nothing to attach to yet; and removing
  the cells as dead code, turned down because they are pure, tested,
  and this phase gives them their consumer.

### Phase 5 — KI-068's timing

- **C19** [NEEDS CLARIFICATION: the recall-hit semantic — does a hit
  mean "surfaced to the model" (move the bump and the row to after the
  first successful response) or "surfaced by assembly" (keep the
  timing and add an answered-event row beside the attempt row)?
  Recommended default: move them, so the history channel and the stream
  count recalls the model actually saw — the attempt row keeps no
  information the turn row does not already carry. The alternative is
  recorded, not invented.]
- **C20** Resolved as the default: the bump and the row fire after the
  first successful model response; a run whose API call fails before
  any response bumps nothing and rows nothing (verified: a test that
  fails the call and asserts no bump and no row; the existing
  assembly-time tests are re-based deliberately).

### Phase 6 — session recall's count store

- **C21** A recalled session bumps a count store beside the archive —
  `<sessions_dir>/.recall-counts.json` on the recall-index's own
  discipline (keyed, atomic, never errors) — and never the session
  log, which stays append-only (verified: a recall against a scratch
  sessions dir bumps the store and leaves every log byte-identical).
- **C22** The hit lands a `recall_hit` row naming the session id,
  reusing the stream's own op with the fact field's meaning widened to
  a memories stem or a session id — no new op in the shared devdec
  vocabulary (verified: the row lands and the calib reader keeps it
  with its shape; the field's doc widened).
- **C23** The synthetic fact stays read-only by construction: the
  store is the only writer of a session's count, and `Memory::
  recall_hits` is still never routed a synthetic fact (verified: the
  existing guard test green).

## Design

**The honest baseline** (what already exists, with evidence, so no
phase rebuilds shipped work):

- The writer's list functions exist and are tested, destructive rule
  included: `src/settings.rs:919-1100`.
- The CLI reaches them: `src/cli.rs:734-771`, `src/main.rs:1255-1309`.
- The pane's commit branches exist but are unreachable:
  `src/tui/app.rs:701-716` against `src/settings.rs:395-433`.
- blend1's doctor report carries the web key; the parse drops it as an
  unrendered extra: the parse test at `src/lanes.rs:1160-1180`, the
  reader at `src/lanes.rs:637`.
- The delegation cells: `src/delegate.rs:1543-1870`; consumers: none in
  `src` (their tests and `src/eval.rs:3678-3708` only).
- The divergence: `src/delegate.rs:1608-1610` versus
  `src/calib.rs:1956` — addressed in phase 4, not ignored.

**Per-phase notes:**

- *Phase 1.* `settings::states` keeps the scalars; a sibling source
  reads the provider roster and the workload table as card states, and
  the Settings mode renders scalars first, then the two list sections.
  The editor seeds from whichever entry was picked; `commit_setting`'s
  existing branches become reachable unchanged. The destructive
  confirmation is a new step in the state machine naming exactly what
  will be removed, answered `y` / anything-else. The safety decision:
  the pane is a view over the same writer, so the destructive rule
  cannot diverge between surfaces — its source is the compiled-in
  roster read from the crate, not a copy.
- *Phase 2.* The check is a pure evaluator over gathered facts in
  `doctor.rs`'s existing pattern: the key resolved by name through the
  run's own precedence, blend1's resolution state beside it. The
  deeper truth — the lane's own report — is what `fleet lanes` renders
  once the parse keeps the web key. The safety decision: a key NAME is
  named, a value never is, per the standing credential rule; and an
  unkeyed optional leg is an informational line, not a failing check,
  so doctor's exit-code contract does not move.
- *Phase 3.* Comments and prose only, each corrected against the
  constant it drifted from. The doc-channel rule applies while writing
  these: the stale strings are named in plain prose here, never in
  backticks, so the nomenclature guard's derived vocabulary is not
  re-established by the doc that reports the drift.
- *Phase 4.* One line of semantics on the delegate side (exclude waived
  from the fit sample), tests pinned on both sides, and the consumer:
  the delegation surface renders the cells' influence beside the graded
  rows — which cell, for which brief shape, and the clause that names
  it, the way the router's walk carries its calibration clauses. The
  backward reach: the delegate cell's fit counts move, so its count
  assertions are re-based deliberately.
- *Phase 5.* Pending the marker. If the default wins, the ordering
  change is in `prepare_memory`'s caller: assemble and render, make the
  request, and only after the first successful response record the hits
  and bump the counts. The in-memory bump's purpose — the next recall
  in this process re-ranks — is preserved because the bump still
  happens inside the run, one turn later. A run that dies before any
  response leaves the corpus untouched, which is the point.
- *Phase 6.* The store is new state beside the archive, on the
  recall-index's discipline; the row reuses `recall_hit` with the fact
  field widened (a memories stem or a session id) so the shared devdec
  vocabulary does not grow an op for one beat's feedback. The safety
  decision: the session log is append-only and the synthetic Memory is
  read-only by construction — both properties survive unchanged, which
  is why the count lives in its own file rather than in either.

**Shared-file seams for parallel implementation:** `src/main.rs` is
touched by phases 2, 4, and 6 — sequence those, or give one implementer
ownership of main.rs's wiring across the three. `src/tui/*` is phase
1's alone. Phase 3 conflicts with nothing. `src/eval.rs` is touched by
phase 4 (and its tests by 5) — one owner.

## Files to be touched

Modified:

- Phase 1: `src/settings.rs`, `src/tui/app.rs`, `src/tui/ui.rs`,
  `src/tui/events.rs` (and their in-file tests)
- Phase 2: `src/doctor.rs`, `src/lanes.rs`, `src/main.rs` (and their
  in-file tests)
- Phase 3: `src/calib.rs`, `src/eval.rs`, `src/graph.rs`,
  `services/README.md` (comments and prose only)
- Phase 4: `src/delegate.rs`, `src/delegate/tests.rs`,
  `src/main.rs`, `src/eval.rs`
- Phase 5 (if the default wins): `src/agent.rs`, `src/memory.rs`'s
  hit-recording tests
- Phase 6: `src/session_recall.rs`, `src/calib.rs` (the field doc),
  `src/main.rs` (the `sessions --recall` wiring), tests

New: none.

## Out of scope

- The router-enforcement plan's one open leg (a live run where the real
  sys1 judges a task `composed`) — a different plan, still in progress.
- A structured-fields editor for `provider.add` in the pane: the JSON
  buffer with the writer's own validation is the honest minimum now;
  the friendly editor is a named follow-up, and the owner's standing
  preference for structured fields over raw JSON is recorded here
  rather than ignored.
- The web tool's `fetch` / `max_urls` forwarding — the lane's own
  bounds are the real clamp and the schema says so.
- The rest of the open ledger (KI-066's unattended-smoke hardening,
  KI-092's cron lost-update window, and the others) — not findings of
  this dive.
- The TUI pane's first human open — disclosed in the 2026-10-15 recap
  as the interactive close; this plan's tests pin behavior and render,
  and the human open remains the experience's proof.

## Verification

- The gate, locally, exit code read directly (KI-014):
  `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo build --release`, `cargo test`. rustfmt and clippy are absent
  on this machine — style matched by hand and disclosed per increment,
  the standing rule.
- Phase 1: the TUI test suite (state machine + TestBackend); the CLI's
  settings tests unchanged.
- Phase 2: live `fleet doctor` and `fleet lanes`.
- Phase 3: grep each corrected prose against the constant it names.
- Phase 4: `cargo test --lib delegate`; the eval corpus's
  `delegation-loop-closes` scenario; a live `fleet delegate --dry-run`.
- Phase 5: the failed-call test asserting no bump and no row; the
  re-based assembly-time tests.
- Phase 6: `fleet sessions --recall` against a scratch sessions
  directory; the logs' bytes identical before and after.

## Linked artifacts

- [docs/architecture/deep-dive-2026-10-15.md](../architecture/deep-dive-2026-10-15.md)
  — the findings this plan addresses
- [docs/plans/2026-10-15-settings-surface.md](2026-10-15-settings-surface.md)
  — C1 and C4's unbuilt halves, closed by phase 1
- [docs/plans/2026-10-15-web-tool-for-every-agent.md](2026-10-15-web-tool-for-every-agent.md)
  — C6, closed by phase 2
- [docs/plans/2026-10-15-session-archive-recall.md](2026-10-15-session-archive-recall.md)
  — C7, closed by phase 6
- [docs/known-issues.md](../known-issues.md) — KI-068 (phase 5),
  KI-092 untouched here

## Risks

- Phase 1 changes interactive behavior on a surface no human has
  opened yet — mitigated by the state-machine and TestBackend tests,
  with the human open named as the close rather than assumed.
- Phase 5 changes the feedback loop's semantics — hence the marker;
  the default is stated for the owner to accept or redirect, not
  assumed.
- Phase 6 touches the archive's read-only-by-construction property —
  the count store is new state beside the archive, never a log write,
  and the guard test stays green.
- The gate cannot run fmt or clippy on this machine — every increment
  discloses it, and CI is disarmed by budget, so the local gate is the
  only gate.
