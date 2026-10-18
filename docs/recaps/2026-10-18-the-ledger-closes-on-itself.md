# The ledger closes on itself, and a flake gets a name

Status: complete
Date: 2026-10-18

Branch: `master`. One session, one plan: the KI triage. The GUI revamp's
last increments (G6–G10) landed at the top of the day and are named in
the commits table; the rest of the session is the triage of
`docs/known-issues.md`, which went from 84 open rows to eleven.

The arc: the owner said "let's fix batch-3" after batches 1 and 2, and
batch 3 turned out not to be a batch of fixes at all. It was a batch of
decisions — and the honest work was naming each one in the row rather
than writing code that nobody had asked for. Five rows did want code.
One defect the gate found wanted code nobody had planned.

## Batch 3 was decisions, not fixes

The plan had sorted 84 open rows into three batches, and the sort was
right: batch 1 and batch 2 were fleet's own defects, and batch 3 was
everything that is not fleet's to decide. Closing those rows well meant
resisting the urge to build. The workload map is not retuned without
graded outcomes to retune it from — and there is no calibration stream on
this machine, so the row's own trigger has not fired. The `?` in
`dev-decisions log`'s text renderer stays, because it is honest. The
gitignored `memories/.working/` subtree stays unbuilt, because one
substrate is the ruling and the diff noise already has an answer.

Five rows did want code, and each was small:

- **KI-050** — the demotion's failure streak read one day's rows, so a
  provider that failed three times a day, every day, started fresh each
  morning and paid nothing. The read now spans the newest three days
  that exist under the stream root, in chronological order, with a serve
  anywhere in the window still resetting the whole streak.
- **KI-019** — the streamed response's `finish_reason` was `None`
  whenever a provider carried the reason on the choice rather than inside
  the usage object, so the streamed path disagreed with the
  non-streaming one. It is derived from the assembled message now.
- **KI-022** — both faces created their session log at launch, so
  opening and quitting left an "open" session holding only a `start`
  record. The log is created inside the first run, and the id rides back
  on a new `UiEvent::Session`; the status bar reads a dash until one
  exists, which is the honest answer to "which session is this?" before
  there is one.
- **KI-076** — the row's second named option, taken: the `subagent`
  declaration now states that `max-turns` is the only bound on the
  delegation as a whole, so a long delegation reads as a long occupancy
  of the parent's turn rather than as an unbounded thing.
- **KI-013** — the fork doc called the kit "zcode-router-kit" while the
  repo is `agnostic-router-kit`, and the lineage was undocumented. The
  kit's own `CLAUDE.md` answers it: the kit is the neutral core
  *extracted from* `zcode-router-kit` on 2026-10-04, and the ZCode
  edition is one of its consumers. The doc now says so, and the
  extraction is why this repo's roster entry for the kit is gone entirely
  rather than renamed.

## The flake the gate found, and the four hours it cost to name

KI-227 is new, and it is the reason this recap exists. Three
`fleet memories` integration tests failed under the full suite's parallel
load — reading 0 facts — and passed in isolation, which is the profile
the repo has been bitten by three times (KI-027, KI-119, KI-204). The
temptation was to file it as the fourth instance of the same family and
move on. It is not the same family.

The evidence that separated them: the panic leaves the scratch corpus
behind, and replaying the *preserved failing corpus* against the same
binary read 6 facts. So the fixture was fine. Instrumenting
`resolve_root_from` showed the child's cwd and home were both correct.
That left one possibility — the root it *chose*. And there it was:
`resolve_root`'s home-first rule prefers `~/.fleet/memories` the moment
that directory holds any `.md`, so any competing corpus — a
consolidator's summary, a `remember` write, the owner's own fleet
process, a second session in the same tree — silently shadows a test's
fixture. The tests had been passing because the ambient home happened to
be empty.

The fix is one line per child: the fixture hands back a `FLEET_HOME`
every child carries, so the home-first rule lands on the fixture's own
corpus by construction rather than by luck. The pinning test writes a
competing corpus into a scratch `~/.fleet/memories` and provably fails —
1 ambient fact instead of the fixture's 6 — when the pin is removed,
which is the difference between a test that passes and a test that
guards. Ten consecutive full-suite runs are clean.

## The ledger's own verdict

The sweep's claim has to be the parser's claim and not a hand count, and
for one commit it was not. `scripts/ki_ledger.py` reported 12 open rows
while the batch-3 commit claimed 217 of 227 closed. Two causes, both
worth recording because both are invisible to the checks that should
have caught them:

- **KI-128's fix landed in `ddb9152` while the row still read `open`.**
  The code was right and the ledger was wrong, which is the failure mode
  a hand count cannot see. The row is closed now, and the posture it
  names — the SIGPIPE restore at the top of `main` — is pinned by a test
  that reads one line then closes the pipe and fails with the row's own
  panic signature when the restore is removed.
- **Fourteen rows carried an unescaped pipe.** The `— moved to batch 3 —`
  annotations the earlier sweeps wrote as a new cell instead of inside
  the next-action cell. The parser pads shifted rows, so no field read as
  missing and the field checks passed — the shift moved the *status*
  read instead, which is exactly what made the two counts disagree. A
  check that pads is a check that hides.

The parser now reports zero problems, and the truth is 216 of 227 closed
with eleven open. Eleven, not ten: KI-207 — the cua driver binary against
the running daemon's contract — was in the file the whole time and no
hand count had found it.

## The gate

Four legs, read directly: `cargo fmt --all --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo build --release`, `cargo test`.
Green at every increment; 1065 tests in the lib alone. The one lint this
round was mine — `nonminimal_bool` in the new SIGPIPE test, fixed as
rustfmt and clippy asked rather than allowed.

## Commits

| Hash | Message |
|------|---------|
| `3a7b471` | fix(ki): the ledger's own verdict — KI-128 closed, fourteen pipes rejoined |
| `4ef5cae` | test(gui): the bridge's test seams carry the session they anchor |
| `34d694b` | docs(ki): the batch-3 sweep — the decisions are named, and ten rows are left outside this repo |
| `aa1643c` | fix(tui,gui): the session log is created on the first task, not at launch |
| `dafc312` | fix(api): the streamed response's finish_reason is no longer None |
| `9fed9ec` | fix(calib): the demotion's streak accumulates across days |
| `d81ad78` | docs(ki): KI-062's closure lands in the ledger |
| `ddb9152` | fix(ki): SIGPIPE and the fact lock — KI-128 and KI-062 closed |
| `8b48f55` | fix(ki): the subagent declaration's cost is measured — KI-071 closed |
| `5cacc36` | fix(ki): the store's mutations run under the lock — KI-092 closed |
| `60aa1f1` | fix(ki): the TUI measures display cells — KI-025 closed |
| `eb2171f` | fix(ki): the pre-flight names the missing keys — KI-045 closed |
| `efd874a` | fix(ki): the runtime is built per command — KI-023 closed |
| `0cfdd3e` | fix(ki): the cron manifest lands repo-side — KI-161, KI-221, KI-224 closed |
| `c7d3086` | fix(ki): the adopted root is named — KI-099 and KI-106 closed |
| `51225b1` | fix(ki): the doctor's roster check sets the gate flag — KI-134 closed |
| `22898de` | fix(ki): the cron cluster — four rows closed |
| `7779e6a` | docs(ki): the last eight batch-2 rows are closed — batch 2 done |
| `5a9213b` | fix(ki): the TUI trio — three rows closed |
| `0ea9c01` | fix(ki): the cadence reads only the active board — KI-118 closed |
| `c7f543b` | docs(ki): KI-017 closed — tool execution leaves the async context |
| `28cd063` | fix(ki): sys1's availability is cached — KI-044 closed |
| `dcd2fdc` | fix(ki): the probe cache and the summary read — two rows closed |
| `c8b2fd5` | docs(ki): batch 2's sweep recorded — eleven closed, nine moved, eight remaining |
| `4d6514f` | fix(ki): two more batch-2 rows closed |
| `953cf98` | fix(ki): two more batch-2 rows closed |
| `2f4cfe9` | fix(ki): the batch-2 quick set — four rows closed |
| `24d67b6` | docs(ki): batch 1 is complete — the triage plan records the sweep |
| `65aea54` | fix(ki): the last batch-1 group — three rows closed, and batch 1 is done |
| `15936bd` | fix(ki): the cron group — KI-089 closed |
| `43921d3` | fix(ki): the lanes group — KI-148 closed |
| `0cce8aa` | docs(ki): KI-030's closure lands in the ledger |
| `432c320` | fix(ki): the lanes group — KI-030 closed |
| `3a84ac7` | fix(ki): the explain trio — three rows closed |
| `25c9883` | fix(ki): the quick group — five rows closed |
| `daaa108` | docs(ki): the docs group — six rows closed |
| `0fa3630` | fix(ki): the compaction group — two rows closed |
| `ce69ae7` | fix(ki): the skills group — two rows closed |
| `bf8ebcb` | fix(ki): the memory group — five rows closed |
| `68a7bd9` | chore(memories): the recall bookkeeping lands as its own increment |
| `9060d79` | fix(ki): the triage's first pass — five rows closed, all 84 open rows batched |

The GUI revamp's G6–G10, at the top of the day: `0a7a86f`, `f63e585`,
`ebec815`, `a5dbb04`, `3a0e454`.

---

## What was added

- `UiEvent::Session { id }` — the event that names the log the agent
  thread created, so both faces can show the session they are logging
  into and nothing before one exists.
- `calib::DEMOTION_WINDOW_DAYS` and `read_days_recent` — the cross-day
  demotion window, read as a sorted listing of the day directories rather
  than date arithmetic.
- `a_closed_stdout_pipe_is_a_quiet_exit_not_a_panic` and
  `a_competing_home_corpus_does_not_shadow_the_fixture` — the two guards
  this round's findings deserved.

## What was fixed

- The demotion's per-day streak, the streamed `finish_reason`, the launch
  session, and the SIGPIPE panic — KI-050, KI-019, KI-022, KI-128, in the
  order the ledger names them.
- KI-227, the test-isolation defect the gate found: three `fleet
  memories` tests read 0 facts under parallel load because the
  home-first resolution rule preferred the ambient `~/.fleet/memories`
  over their fixture.
- Fourteen ledger rows whose next-action cell had been split by an
  unescaped pipe, and one row (KI-128) closed in the commits but not in
  the ledger.

## What was changed

- The `subagent` declaration now states that `max-turns` is the only
  bound on the delegation as a whole.
- The fork's router doc names the kit's lineage.
- The KI triage plan records all three sweeps and the parser's verdict.

## Files changed

**Core**
- `src/calib.rs` — the cross-day demotion window and its test
- `src/api.rs` — the derived `finish_reason`
- `src/agent.rs` — `UiEvent::Session`
- `src/tui/mod.rs`, `src/tui/app.rs`, `src/tui/ui.rs` — the lazy session log, its event, and the status bar's dash
- `src/gui/bridge.rs`, `src/gui/mod.rs`, `src/gui/assets/app.js` — the same on the browser face
- `src/memory.rs` — KI-062's fact lock, KI-087's `holds_facts`, KI-061's load-time sanitize, KI-059's protected synonyms
- `src/cron.rs` — KI-089's atomic claim, KI-093/094/095's clock guards, KI-105's eval minute
- `src/route.rs` — KI-045's pre-flight, KI-044's sys1 guard
- `src/boards.rs` — the parse-only extensions and the active-board cadence
- `src/session.rs`, `src/checkpoints.rs`, `src/compaction.rs`, `src/tools.rs`, `src/main.rs`, `src/improve.rs`, `src/lanes.rs`, `src/mcp.rs`, `src/session_recall.rs`, `src/eval.rs`, `src/explain.rs`, `src/delegate.rs`, `src/home.rs`, `src/lane_client.rs`, `src/lib.rs`

**Tests**
- `tests/integration.rs` — the memories fixtures' `FLEET_HOME` pin and the shadowing regression test
- `tests/install.rs` — the closed-stdout-pipe guard
- `tests/blend1.rs`, `tests/eval.rs`, `tests/improve.rs` — the suite's share of the row fixes

**Documentation**
- `docs/known-issues.md` — the ledger: 84 open rows to eleven
- `docs/plans/2026-10-17-ki-triage.md` — all three sweeps and the parser's verdict
- `docs/plans/2026-10-16-fleet-routes-through-its-own-router.md` — the kit's lineage
- `docs/plans/2026-10-16-gui-revamp.md` — closed, with G6–G10 recorded
- `docs/features/gui.md`, `docs/features/boards.md` — the revamp's surfaces
- `README.md`, `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md` — the revamp's contract pass

---

## Doc updates applied

- `docs/plans/2026-10-16-gui-revamp.md` — status `completed`, G6–G10 recorded with their SHAs
- `docs/plans/2026-10-17-ki-triage.md` — the three sweeps, the batch-3 decisions, and the parser's verdict
- `docs/known-issues.md` — the closures themselves, each naming its pinned test
- `docs/features/gui.md`, `docs/features/boards.md` — the revamp's surfaces (earlier in the day)

## Doc updates deferred (debt)

Two contract docs now describe behavior this session changed. Proposed
text below; not applied, because applying a contract-doc update is the
owner's call and this session's job was the recap.

### `docs/features/calibration.md` § "The first reader — bounded demotion"

The section's first bullet says "Only today's file". That is no longer
true. Proposed replacement for the bullet:

```markdown
- **The newest `DEMOTION_WINDOW_DAYS` (3) days' files**, in
  chronological order. The day directories are
  `<root>/<YYYY>/<MM>/<DD>/events.jsonl`, whose path order *is*
  chronological order, so the read is a sorted listing rather than date
  arithmetic — and a machine idle for a week reads its last active days
  rather than a week of empty files. The day the caller names is always
  read, even when it is not among the newest.
```

and for the second bullet, one added sentence:

```markdown
  The streak accumulates across the window's days, so a candidate that
  fails three times a day, every day, escalates instead of starting
  fresh each morning (KI-050); a `served` attempt anywhere in the window
  resets the whole streak.
```

### `docs/features/subagents.md` § the `max-turns` bound

The schema's description and the surrounding prose carry the cap's
numbers but not the fact that it is the only whole-delegation bound.
Proposed addition after line 66:

```markdown
`max-turns` is the only bound on the delegation **as a whole**. Each
model call is bounded by the transport timeout and each tool by its own
deadline, but nothing caps the wall clock the subtree may occupy — so a
long delegation is a long occupancy of the parent's turn, and the
declaration says so rather than leaving the reader to assume a deadline
that does not exist (KI-076).
```

### `TECHNICAL-DOCUMENTATION.md` — no update needed

The session's core changes (a wider read window, a derived field, a
lazier session, a restored signal disposition) do not move any module
summary the document carries.

### `FUNCTIONAL-SPECIFICATIONS.md` — no update needed

Two of the changes are user-visible in a small way and both are already
covered: the session log's timing is an internal seam (a launch that
never ran leaves no log, which is the absence of a thing rather than a
new flow), and the SIGPIPE posture is the conventional CLI behavior the
document already assumes for a piped run.

---

## Open questions / next steps

- **KI-117 is the owner's call and it is the only one that blocks a
  decision inside this repo**: `serde_json`'s `preserve_order` pulls
  `indexmap`, a genuinely new crate, and the zero-new-crates contract
  makes adding it the owner's. The tradeoff is named in the row; no
  surface is observed to need the payload's own order.
- The eleven open rows are all outside this repo or waiting on the
  owner: KI-001 (the kit's CI, diagnosed), KI-003/029 (upstream sys1's
  uncommitted files), KI-004 (gen1 has no remote), KI-005 (sem1's
  missing `CLAUDE.md`), KI-066 (the smoke pattern's fail-closed port
  check), KI-117, KI-135 (the remaining lanes' live calls), KI-136 (the
  Linux quick start), KI-143 (filing blend1's MCP adapter upstream),
  KI-207 (the cua driver against the running daemon's contract).
- The triage plan is marked `completed`: all eight acceptance criteria
  are met and each was verified rather than assumed — C0 by the parser's
  zero-problem verdict, C5 by ten consecutive clean runs, C7 by the
  eleven open rows each naming their action. The owner may still want to
  read the batch-3 decisions in the ledger, because a row that names a
  decision is a row the owner can overrule.

## Notes

- The four hours KI-227 cost were the session's best-spent hours. The
  repo already had three flakes filed as "environmental, machine load",
  and this one matched their profile exactly — passes in isolation,
  fails under load, different tests each time. What separated it was
  replaying the preserved corpus against the same binary before
  theorizing, which is the KI-204 lesson applied on purpose rather than
  in hindsight.
- A check that pads is a check that hides. `scripts/ki_ledger.py` fills
  short rows with empty strings so the field checks can run, which means
  a row whose cells have shifted reads as a *shifted* row rather than a
  broken one — and the count it reports moves with the shift. The cell
  count and the field checks should disagree loudly, or the padding
  should refuse.
- The two faces now create their session log in the same place for the
  same reason, which is the first time the TUI's agent thread and the
  bridge's have had to agree on a seam this session introduced. The
  agreement is structural (both arms create inside the first Run), not
  coincidental.
- `tmp/` holds fourteen files of session scratch — the vm harnesses and
  their debug scripts — and is untracked but not gitignored. The
  documented practice is that it is never committed; making the ignore
  explicit would enforce that with the tool rather than with memory, and
  was left alone as out of scope.
