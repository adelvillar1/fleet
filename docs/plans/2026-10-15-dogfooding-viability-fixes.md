# The dogfooding viability fixes — the second dive's five, plus the toolchain that makes the gate whole

Status: complete — implemented 2026-10-15/16; every criterion graded in
the Convergence section below.
Date: 2026-10-15

## Why

The owner's goal: use Fleet to do real work — including building its own
still-missing parts — and judge the harness by that use. The second deep
dive (this session, 2026-10-15, findings delivered in chat; the first
dive's plan completed at `a2555fe`) found the harness viable for
attended work today and named the short list between "works" and
"works all day": five defects, two of them unattended-mode approval
bypasses, plus one environment repair that currently dead-ends the
self-maintenance path. This plan fixes exactly that list. Everything
else the second dive found is recorded under Out of scope for later
triage, not dropped.

**CI stays as it stands**: disarmed by budget, the gate local-only
(KI-006). The owner is considering a self-hosted Gitea instance
(Railway) as a possible future gate host and has not decided; this plan
changes nothing about CI, and no phase depends on it.

## The findings this plan fixes

| ID | Finding | Evidence |
|---|---|---|
| KI-211 | The bash wall-clock cap does not bound the call: the timeout loop runs only while `sh` is alive, and both exits `join()` drain threads blocked in `read_to_end`, which returns only when every process holding the inherited pipe write-ends exits — a backgrounded descendant (`nohup … &`, a dev server) hangs the tool forever, and on the timeout path `kill()` reaps only `sh`, so the joins hang too. `drain` also swallows mid-read errors while reporting `truncated: false` | `src/tools.rs:816-871`, `:890-896` |
| KI-209 | `yes-guarded` never enforces the destructive class on a forwarded subagent bash call: the child's call arrives renamed `subagent → bash` (`src/subagents.rs:1844-1860`), but `bash_command` keys on the exact name `bash` (`src/approvals.rs:182`) so `is_destructive` returns false (`:248-253`) and the mode auto-approves (`:230`). The gate's own purpose — the destructive class stops the unattended run — does not survive one level of delegation | `src/approvals.rs:181-187`, `:220-232`, `:248-253`; `src/subagents.rs:1833`, `:1844-1860`; `src/main.rs:966` |
| KI-210 | The always-ask class dies at delegation depth 2: each delegation level applies `labelled()` once, so a grandchild's `computer_use` reaches the root gate as `subagent → subagent → computer_use`; `asks_regardless` strips the prefix exactly once (`:602-608`), misses `ALWAYS_ASK_TOOLS`, and under `yes`/`yes-guarded` the desktop tool runs with no prompt. The owner's 2026-10-15 ruling — it asks in every mode — does not survive one extra hop | `src/approvals.rs:595-608`; `src/subagents.rs:1833`, `:1851`; `src/tools.rs:135`; `src/subagents.rs:272-283` |
| KI-208 | A committed-turn failure is recorded against the wrong provider: in the committed path `start` is a walk position (`state.index = position`, `:1739`) but the failure record indexes the roster with it directly — `self.resolved[start]` where every other record maps `self.order[position]`. Whenever the walk order is not roster order (the normal judged-walk shape), the record names whoever sits at that position, and that record feeds `calib::demoted_providers`' streak — the wrong provider is demoted for another's failures, reordering future walks | `src/route.rs:1684-1696` against `:1700-1701`, `:1731-1750`, `:1739`; `src/calib.rs:1593-1618` |
| KI-212 | Consolidation can stamp facts as superseded by a summary that was never written: `write_summary` discards its own write result (`let _ = write_document(...)`), and the caller then unconditionally stamps `superseded-by: <summary>` and lands compress/supersede rows. On a write failure the fold retires every source fact from recall with a pointer to a nonexistent file, and the audit trail claims preservation that did not happen | `src/memory.rs:3791-3793`; `:3680-3706` |
| KI-213 | One torn trailing JSONL line bricks the whole session: `record` write_alls up to 64 KiB per line, and `replay` fails the entire log on the first unparseable line — a SIGKILL mid-record makes every intact record before it unreadable, so resume fails and `fleet sessions` marks the log `unreadable` | `src/session.rs:230-236`, `:241-256` |

The environment finding is already on the ledger and is fixed here
because it gates everything else: **KI-007** — no rustup shim, so
rustfmt and clippy are absent, the gate's first two legs cannot run,
and the improve gate's source-apply path fails closed on this machine
(KI-097, KI-104).

## Acceptance criteria

### Phase 0 — the ledger rows

- **C1** KI-208 through KI-213 are filed open in `docs/known-issues.md`,
  each with its path:line evidence and failing shape, before any fix
  commit — the fixes close rows rather than create them late.
- **C2** No behavior change in this phase (docs-only increment, explicit
  paths).

### Phase 1 — the toolchain (KI-007)

- **C3** A rustup shim resolves on PATH (brew's `rustup` or
  `rustup-init`, whichever the owner prefers) and
  `rustup component add rustfmt clippy` succeeds against the existing
  stable toolchain — no toolchain upgrade, no profile change.
- **C4** `cargo fmt --version` and `cargo clippy --version` resolve with
  the CLAUDE.local.md PATH line, and CLAUDE.local.md's "no rustup
  shim" quirk is updated in the same turn.

### Phase 2 — the gate becomes whole (the first fmt/clippy pass)

- **C5** `cargo fmt --all --check` runs and the tree passes;
  formatting remediation lands as its own increment with explicit
  paths.
- **C6** `cargo clippy --all-targets -- -D warnings` runs clean — every
  lint fixed in the crate's own voice or annotated with a named
  reason, never a blanket allow.
- **C7** `cargo build --release` and `cargo test` stay green through
  the remediation, and the standing gate is now all four legs; the
  recap discloses that the gate runs whole.
- **C8** The improve gate's fmt/clippy steps become runnable on this
  machine — a source proposal's gate can in principle pass (verified
  by inspection or a dry probe of the gate steps, not by filing a real
  proposal). KI-097/KI-104's premise is updated to name the machine as
  repaired.

### Phase 3 — the bash cap bounds the call (KI-211)

- **C9** A bash call whose `sh` exits while a background descendant
  holds the pipes returns within a bounded grace — the output carries
  what drained, marked `truncated`, naming the cut (a background
  process still holds the pipe), never hangs. Integration test: a
  script that backgrounds `tail -f /dev/null` and exits; assert the
  call returns inside a generous bound with the named cut.
- **C10** The timeout path returns the same way when a descendant
  survives `sh`'s kill — the 120s cap plus the grace is the whole
  ceiling. Integration test: a backgrounded long sleeper plus a long
  foreground sleep; assert a bounded return after the kill.
- **C11** A mid-read pipe failure is no longer presented as clean
  output: it is named on the result (the every-error-logged rule and
  the truncation-marker contract).
- **C12** No new dependency: the fix stays std-only (see Design for the
  rejected alternative).

### Phase 4 — the gate judges the real call across delegation (KI-209, KI-210)

- **C13** `Gate::is_destructive` reads a forwarded `subagent → bash`
  call's inner command through the label: a destructive inner command
  is named destructive, a benign one is not (unit tests on both signs
  at the Gate level, yes-guarded prompting pinned through the state).
- **C14** `asks_regardless` survives any number of prefix repetitions —
  `subagent → computer_use` and `subagent → subagent → computer_use`
  both ask in every mode (unit tests for the depth-1 and depth-2
  shapes).
- **C15** The benign paths are unchanged: yes-guarded still auto-approves
  a forwarded non-destructive bash call, `--yes` behavior for
  non-forwarded calls is byte-identical, and the existing approval
  tests (the always-ask forwarding test, the three-mode table) stay
  green.
- **C16** The labelled call the owner sees in the prompt keeps its shape
  (goal + tool named) — the fix lives in the gate's reading, not the
  label's composition.

### Phase 5 — the failure record names the provider that failed (KI-208)

- **C17** The committed path's failed attempt names the committed
  candidate — read through the walk order — unit-tested with a walk
  whose order is not the identity (the judged-walk shape), asserting
  the record's provider equals the candidate `one_attempt` actually
  dialed.
- **C18** A demotion read over a scripted day with a committed mid-stream
  failure credits the failing provider, not the position's roster
  entry — the calibration-side assertion, so the loop's input is
  pinned, not just the record.
- **C19** No other record path changes: the walk's served/failed/skipped
  records already map through the order, and the existing route tests
  stay green.

### Phase 6 — the fold stamps only what landed (KI-212), and the recall-hit seam closes

- **C20** `write_summary` propagates its write result; on failure the
  cycle stamps no `superseded-by`, lands no compress/supersede rows
  for that fold, and the source facts stay live and rankable — the
  failure is named on the cycle's own audit path, not silent. Unit
  test: a failing write (an unwritable target) asserts no stamp, no
  rows, facts intact.
- **C21** On success the behavior is byte-for-byte today's (the existing
  consolidation tests green unchanged).
- **C22** `record_recall_hits` fires only for a response that yielded a
  choice — moved below the empty-choices check — so a degenerate
  zero-choice 200 bumps nothing and rows nothing (unit test beside the
  KI-068 fix's existing tests; the failed-call tests stay green). This
  is the one residue of KI-068's closure, named for exactly what it
  is: a one-line honesty correction, not a re-litigation.

### Phase 7 — a torn tail cannot brick a session (KI-213)

- **C23** `replay` of a log whose final line is torn (unparseable,
  trailing newline absent) returns every intact record and names the
  tolerated tail — the session resumes. Integration test: a good log
  plus an appended half-record; replay succeeds with the intact
  records and resume seeds from them.
- **C24** A malformed line anywhere but the last still fails loud,
  naming file and line — the discipline that catches real corruption
  does not loosen. Test: a garbage mid-log line still refuses.
- **C25** A final line that parses as a complete record is kept, not
  dropped — the tolerance is for the torn, never for the valid.
- **C26** `fleet sessions` renders a torn-tail session readable (its
  state derives from the tolerated replay), with the count excluding
  the torn tail.

## Design

**The honest baseline** (verified at HEAD `a2555fe` unless named):

- The drain/timeout shape to fix: `src/tools.rs:816-871` — timeout loop
  on `try_wait`, both exits `join()` two threads blocked in
  `read_to_end`; `BASH_TIMEOUT` is 120s (`src/tools.rs:20`).
- The gate's two readers: `bash_command` (`src/approvals.rs:181-187`)
  and `asks_regardless` (`:602-608`); the label's composition:
  `labelled` (`src/subagents.rs:1844-1860`), applied once per
  delegation level at `:1833`.
- The committed path: `src/route.rs:1684-1696` against the walk's own
  position→roster mapping at `:1700-1701`.
- The fold: `write_summary` ends `let _ = write_document(...)`
  (`src/memory.rs:3793`); the unconditional stamps and rows at
  `:3680-3706`.
- The reader: `replay` fails on any unparseable line
  (`src/session.rs:241-256`); `record` write_alls one line and flushes
  (`:230-236`).
- The toolchain: rustup `profile = "minimal"`, no shim (KI-007); the
  PATH line in CLAUDE.local.md is the one place the toolchain is
  named.

**Per-phase notes:**

- *Phase 3.* The drains become chunked senders over a bounded collect:
  each drain streams what it reads through a channel, and after the
  child is reaped the collector waits a named grace (a constant beside
  `BASH_TIMEOUT`) for each channel, takes what arrived, and on a
  timeout returns it marked `truncated` with the named cut — a
  background descendant may keep the pipe, but it can no longer keep
  the call. The same collect serves the timeout path after the kill.
  **The rejected alternative**, recorded: a process-group kill
  (`setsid` + group signal) would also reclaim the descendant, but it
  needs libc or an unsafe `pre_exec` in a crate whose direct
  dependencies are five and std-only beyond them; the bounded drain
  achieves the bound without either. If a descendant is ever observed
  starving the grace window in real use, revisit with the process
  group.
- *Phase 4.* One `bare(call)` reader in `approvals.rs`: it strips every
  repeated `subagent → ` prefix for name checks, and for a prefixed
  call whose arguments parse as the labelled wrapper it unwraps the
  inner `tool`/`arguments` so `bash_command` reads the command the
  child actually issued. Both `is_destructive` and `asks_regardless`
  read through it; the labelled call itself is untouched, so the
  prompt keeps its shape. The safety decision: the gate judges the
  real call at every depth — the class rules are about what the call
  does, not where it came from.
- *Phase 5.* One expression: the committed path maps its position
  through `self.order[start]` before naming the provider, the same
  mapping the walk loop already applies. The test pins the
  non-identity order because that is the only case the bug fires in —
  an identity-order test would pass on the broken code.
- *Phase 6.* `write_summary` returns the write's `Result`; `consolidate`
  stamps and records only on `Ok`, and on `Err` names the failure on
  the cycle's own path (the advisory-cycle discipline: the pass names
  what it could not do, the sources stay live). The KI-068 residue is
  a one-line move of `record_recall_hits` below the empty-choices
  check, beside the fix's own tests.
- *Phase 7.* `replay` splits its failure by position: an unparseable
  final line (with or without a trailing newline) is dropped and named
  in the return path the caller can surface; an unparseable line
  before it still fails the read with file and line, exactly today.
  The appended-partial-write case is the target; real corruption in
  the middle of a log keeps its loud failure.

**Shared-file seams for parallel implementation:** none across phases —
each phase owns its files (`tools.rs` 3; `approvals.rs` 4; `route.rs` 5;
`memory.rs` + `agent.rs` 6; `session.rs` 7). Phase 2's remediation may
touch any file, which is exactly why it lands first: every code phase
then gates through all four legs on a tree that already passes them.

## Files to be touched

- Phase 0: `docs/known-issues.md`
- Phase 1: CLAUDE.local.md (the quirk line); no repo file
- Phase 2: formatting/lint remediation across whatever the first pass
  names (its own increment, explicit paths)
- Phase 3: `src/tools.rs` (+ its in-file tests, + an integration test
  in `tests/integration.rs`)
- Phase 4: `src/approvals.rs` (+ its in-file tests; a depth-2 labelling
  test may live in `src/subagents.rs`'s tests)
- Phase 5: `src/route.rs` (+ its in-file tests)
- Phase 6: `src/memory.rs`, `src/agent.rs` (+ their in-file tests)
- Phase 7: `src/session.rs` (+ its in-file tests, + an integration test)

## Out of scope

- The second dive's remaining findings, recorded for a later plan:
  the provider pin silently dropped on an unmapped workload;
  `gates.default_workload` accepting any string (the typo path into
  the reserve-serving full walk); the improve gate's fail-silent
  revert cluster (`Restore::apply`, `stage_config`'s write-before-
  stash, `revert_path`, `finish`'s stamp); the consolidator-vs-run
  archive TOCTOU; the role overlay's missing record; the TUI approval
  modal's overlay clobber; the nomenclature trio (`NOMEN_DEBUG` in the
  hot loop, the dead `ies→y` rule, deletions invisible to `fresh`);
  cron's catch-up label; the settings writer's cross-process window
  and the other assumed gaps; episodic summaries written with no
  shape and no phrases (the recallability backlog item).
- **CI**: stays disarmed by budget, the gate local-only (KI-006). The
  owner's Gitea-on-Railway idea is recorded here as undecided — no
  phase plans for it, and nothing here blocks it either way.
- The rest of the open ledger — not findings of this dive.

## Verification

The gate, locally, exit code read directly (KI-014):
`cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --release`, `cargo test` — all four legs from phase 2
onward; phases gated before that run build + test and disclose the
absent legs. rustfmt and clippy absent until phase 1 lands — style
matched by hand and disclosed per increment until then, the standing
rule. Live proofs are the integration tests themselves: the
backgrounded-descendant return (C9), the surviving-kill return (C10),
the torn-tail resume (C23).

## Convergence

Graded against the committed code, phase by phase. Commits, oldest
first: `c7741c9` (phase 0), `3e05dfe` (phase 2's fmt half), `7643cf7`
(phase 2's clippy half), `8a3275e` (phase 3), `d7b93dd` (phase 4),
`328faf5` (phase 5), `36facd6` (phase 6), `9b586dd` (phase 7). The
final gate: `cargo fmt --all --check`, `cargo clippy --all-targets --
-D warnings`, `cargo build --release`, `cargo test` — all four exit 0,
1013 passed / 2 ignored, rustfmt 1.8.0 and clippy 0.1.89 installed
before the code phases began, so every increment gated whole.

- **C1, C2 — met.** KI-208..213 filed open at `c7741c9` with their
  evidence, before any fix commit; docs-only, no behavior change.
- **C3, C4 — met.** brew's rustup shim (keg-only, the path named in
  CLAUDE.local.md), components added against the existing stable
  toolchain; `cargo fmt --version` → 1.8.0, `cargo clippy --version` →
  0.1.89; CLAUDE.local.md's quirk line updated in the same turn.
  KI-007 closed.
- **C5-C8 — met.** The fmt pass (`3e05dfe`, 49 files, no behavior
  change) and the clippy pass (`7643cf7`, 27 files) are separate
  increments; the first clippy run surfaced 64 lints across three
  waves, each fixed in the crate's voice or annotated with a named
  reason (the two `too_many_arguments` counts and the two
  `await_holding_lock` test locks are the annotated ones, each with
  its why beside the attribute); the four-leg gate ran green after
  each; KI-097/KI-104's premises updated to name the machine repaired.
- **C9-C12 — met, with one placement note.** The bounded drain landed
  as designed (chunked channel sends, a 2s `DRAIN_GRACE` collect past
  the reaped shell, the cut named and `truncated: true`, read failures
  named, std-only). The pins live as unit tests in `src/tools.rs`'s
  own module rather than in `tests/integration.rs` — `run_bash` is
  module-private, and exposing it test-only through the public surface
  would widen the crate's API for a test's sake; the substance the
  criterion names (a bounded return, the named cut, both timeout-path
  and holder-path) is pinned by
  `a_background_process_holding_the_pipe_cannot_keep_the_call` and
  `the_deadline_kills_a_hung_shell_and_the_collect_bounds_its_survivors`.
  KI-211 closed.
- **C13-C16 — met.** `unlabel` strips every `subagent → ` repetition
  and unwraps the labelled wrapper's inner arguments; the destructive
  class and the always-ask class both key on the bare call; the
  labelled call the prompt shows is untouched. Pinned by four new
  tests; the existing approval suite green unchanged. KI-209, KI-210
  closed.
- **C17-C19 — met.** The committed path maps its position through
  `self.order` before naming the provider, pinned with a non-identity
  walk order against a closed loopback port, and the recorded attempt
  is fed through `calib::demoted_providers` in the same test, which
  names the dialed provider and only it. The walk's other records were
  already correct; the route suite green unchanged. KI-208 closed.
- **C20, C21 — met.** `write_summary` returns its `Result`; on failure
  the cycle stamps nothing, lands no rows, archives nothing (the
  forget half follows the fold — the one shape the plan's text did not
  spell out, where archiving an unstamped expired fact would have
  lost it), and names the failure on stderr; the read-only-corpus test
  asserts no stamp, no rows, and an untouched fact, then the retry
  folds whole. The success path is byte-identical (the existing cycle
  tests green unchanged). KI-212 closed.
- **C22 — met, with its premise corrected.** The ordering move landed:
  `record_recall_hits` fires strictly after the empty-choices check.
  Writing the test exposed that the plan's degenerate shape — a
  zero-choice 200 — is unreachable on the streamed client:
  `chat_stream_plain` always assembles one choice from a completed
  stream, so a `[DONE]`-only stream is a *completed run with an empty
  answer*, and the hits firing there is correct (the model did
  receive a response). The test lives in `tests/integration.rs`
  (`a_degenerate_stream_is_still_one_answer_and_the_hits_fire_once`,
  on the cross-process mock — an in-process loopback server shares the
  process with the client, the exact shape the mock-server was built
  to avoid) and pins the real behavior: completed, the bump exactly
  one step, exactly one `recall_hit` row. The empty-choices guard
  stays as defense for any future client path.
- **C23-C26 — met.** The tolerance is exactly one shape — an
  unparseable final line on a log that does not end with its newline —
  dropped and named on stderr; a malformed line anywhere else still
  fails loud naming file and line; a parseable final line is kept.
  Pinned by three new tests in `session.rs`. C26's surface reads
  through `session::list`, which derives its states from `replay`, so
  a torn-tail session renders readable with no separate test — the
  replay pins carry it.

## Linked artifacts

- [docs/architecture/deep-dive-2026-10-15.md](../architecture/deep-dive-2026-10-15.md)
  — the first dive; its findings plan completed at `a2555fe`
- [docs/plans/2026-10-15-deep-dive-findings.md](2026-10-15-deep-dive-findings.md)
  — the completed predecessor plan this one follows
- [docs/known-issues.md](../known-issues.md) — KI-007, KI-006, KI-097,
  KI-104, KI-068; KI-208 through KI-213 filed by phase 0

## Risks

- Phase 2's first clippy pass is unbounded work until it runs — budget
  it as its own increment and stop at green; it must not absorb the
  code phases.
- Phase 3 trades perfect cleanup for a bounded call: a living
  descendant keeps its pipes and its life, and the output is cut —
  named, and the process-group alternative is recorded if the grace
  window proves too tight.
- Phase 4 touches the approval gate — the tests are the contract, and
  the benign-path criterion (C15) is what keeps the fix from widening
  what runs unasked.
- Phase 7 loosens replay at exactly one position — the final line; the
  mid-log fail-loud test (C24) is the guard that the loosening does
  not spread.
