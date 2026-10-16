# The dogfooding viability fixes — the second dive's five, plus the toolchain that makes the gate whole

Status: draft — filed 2026-10-15; becomes active when implementation begins.
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
