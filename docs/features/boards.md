# The boards (slice 14)

Three read-only surfaces over this machine's own sources, mounted in the
TUI's board pane: the **router board** over the router kit's operator API,
the **second-brain board** over its six derived state files, and the
**tower** over git facts per project. The visual bar is directive 4 —
cards, status dots, bars, real layout — never a plain-text table.

Spec: the fork's `@fleet/dsh-router-board`, `@fleet/dsh-second-brain`,
`@fleet/dsh-tower`, and `@fleet/dsh-board-seam`. The seam itself is not
built here: it exists to serve the fork's browser face over HTTP, and this
slice's surface is the TUI pane, so the seam's cadence-and-route machinery
has no consumer. Its contract — one cached snapshot per board, refreshed on
a self-rescheduling cadence, a route that never waits on the source — is
ported into `src/boards.rs` as the refresher thread described below.

## The surface

The board pane (slice 3's mount point, 42 columns) shows one board at a
time. **Ctrl+B** cycles router → second-brain → tower; the pane's title
names the active board (` board — router `) and the status bar repeats the
name (` board router `), so the surface is named in two places. Cycling
forces an immediate read of the board it lands on.

No embedded web server ships this slice: the plan's "and/or" allows the TUI
panes, a server needs dependencies this crate does not carry
(`Cargo.toml`/`Cargo.lock` unchanged), and the browser face is a later
surface over the same reads.

## The sources

| Board | Source | What it renders |
|---|---|---|
| router | `GET http://127.0.0.1:8300/api/state`, `/api/usage`, `/api/suggest` | the workload tier chains with their resolved primary and fallback count, the judge's mode, what the kit just served (status dot per call), the usage ledger's counters, and the per-model call bars |
| second-brain | `gates_state.json`, `harness_state.json`, `loops_state.json`, `reports_state.json`, `maturity_state.json`, `tower_data.json` under the second-brain home | the earned-autonomy gate board (verdicts, dispositions, graded progress, fit-ready), the HITL inbox, the harness loops, the quality loops and their graded rows, the maturity sparkline, the doing/conflict counts, the tower's project aggregates |
| tower | `git` per immediate subdirectory of `~/Projects` | per-project cards: branch, dirty dot and count, head commit, the trailing-thirty-day commit count, ahead/behind against the upstream |

Each board is a composed ratatui render of its source's real data — no
board renders a placeholder, a prose paragraph, or a plain-text table.

## The token, and where it comes from

The kit's operator API authenticates with its `localToken`, which lives in
the kit's own rendered runtime config — `<kit home>/router/config.json`,
the file `kit apply` writes. `src/boards.rs` resolves that file the way the
kit's own `lib/paths.mjs` resolves it:

1. `$AGNOSTIC_ROUTER_DIR/config.json`
2. `$AGNOSTIC_ROUTER_KIT_HOME/router/config.json`
3. `~/.agnostic-router-kit/router/config.json` (the kit's documented default)
4. `~/.zcode/router/config.json` — where this machine's running router
   actually renders its runtime, one directory over from the documented
   default. Named rather than assumed.

Two explicit overrides win over the file: `AGNOSTIC_ROUTER_URL` (the base
URL) and `AGNOSTIC_ROUTER_TOKEN` (the bearer). Everything resolves through
the same environment-then-`<FLEET_HOME>/.env` chain every other credential
in this crate uses, and the port the board dials is read out of the same
config file the token comes from. **The token is never written into a
snapshot, a log line, or this repository** — it rides the `Authorization`
header of the request it authorizes and is dropped, and two tests pin the
invariant: one asserts the token string appears nowhere in a snapshot's
serialization or in a refused read's named error, and one asserts it
appears nowhere in the rendered board at either width.

A bad or absent token is a **refused read**, not a crash: the kit answers
401 with `router: bad local token`, the read names that sentence, and the
pane keeps its last snapshot.

The second-brain home resolves as `$SECOND_BRAIN_HOME`, then
`~/.second-brain`, then `~/Projects/second-brain`. The projects root
resolves as `$TOWER_LOG_PROJECTS_DIR` (the tower's own CLI names the same
variable), then `~/Projects`.

## The snapshot contract

Every board keeps the last snapshot that landed. A refused read — a dead
port, a bad token, an unreadable or missing state file, a git failure —
keeps that snapshot visible and names the error at the top of the pane on
one line, with the minute of the last good read beneath it:

```
 ! the kit at 8300 did not answer
   last good read 2026-10-14T09:41Z
```

A **healthy** board names its read time too, on the same line at the top
of the pane (`   read 2026-10-14T09:41Z`). Without it the operator cannot
tell a second-old snapshot from a twelve-minute one, which is the whole
difference between a live board and a stale one.

A board that has never read successfully shows ` no snapshot yet` and the
reason. Inside a snapshot, one read that did not land names itself (the
second-brain board's six files are independent reads: a missing
`maturity_state.json` names that file and the other five still render),
and the router board's `/api/usage` or `/api/suggest` refusal names its
path while `/api/state` — the spine — still renders.

**Shape drift is named, never silent.** The router board's parser reads
`judge.mode`, `guard.allowPayg`, `resolved.workloads`, and `/api/usage`'s
`totals` and rows out of the kit's payloads. A kit upgrade that renames
one of those would otherwise empty the section with no reason given, so
the read names each absence the way the second-brain board names a state
file that is not the shape it claims, and it rides the top of the board:

```
 △ /api/state shape changed
   resolved.workloads is absent
 △ /api/usage shape changed
   0 of 1 model rows could not be read
```

A row the board cannot read is named too (`N of M model rows could not be
read`), rather than dropped in silence.

## The refresh

`boards::spawn_refresher` owns one thread with its own current-thread
tokio runtime (the `spawn_consolidator`/`open_lanes_view` precedent). It
reads all three boards concurrently at start and every
`BOARD_REFRESH_SECS = 30` seconds, and again the moment a
`BoardCommand::Refresh` arrives. The reads run on the runtime's blocking
pool, so the slowest source sets a refresh's cost rather than the sum of
all three.

A refresh asked for while a read is already running is **satisfied by
that read** rather than queued behind it: the command channel is drained
after every read and the cadence restarts from the end of the read, so N
presses of Ctrl+B cost one read, not N — and a read that overruns its
period stretches the cadence rather than firing a second read the moment
the first lands.

## The bounds

Every read is bounded, per-read *and* per-scan, because a sum of bounded
reads is not itself bounded:

| Bound | Value | What it covers |
|---|---|---|
| `READ_TIMEOUT_SECS` | 8 | one operator-API GET (the fork's carrier uses an 8s abort signal) |
| `GIT_TIMEOUT_SECS` | 5 | one `git` call |
| `TOWER_SCAN_BUDGET_SECS` | 20 | the **whole tower scan** — thirty projects × five calls is up to ~12.5 minutes of wall time, and a hung `git` on one repository must not cost the cadence behind it |
| `BRAIN_READ_BUDGET_SECS` | 10 | the **whole six-file second-brain read** |
| `FILE_READ_TIMEOUT_SECS` | 5 | one state-file read, on its own thread, so a file on a hung mount is a named refusal rather than a stalled cycle |
| `MAX_PROJECTS` | 64 | immediate subdirectories per tower scan — a machine with more projects than that names the rest rather than spawning an unbounded number of children |

When the tower scan's budget is spent the scan stops, the projects read so
far render, and the snapshot names the truncation (` 3 of 6 projects` /
`   the scan budget was spent`) — a truncated scan never reads as a
machine with fewer projects than it has. The second-brain read works the
same way: once its budget is spent no further file is read, and each
unread file names the budget. A bounded `git` call that expires is killed
and named; the eval probe's drain-thread pattern keeps a chatty child from
deadlocking the wait. The event loop only ever receives finished reads over
a channel, so a slow or hung source cannot stall the interface.

A read that panics on the blocking pool is **named, never swallowed**: the
board keeps its last snapshot, the pane says the read could not run, and
the refresher's own thread survives — the crate's CatchUnwind lesson from
cron/eval, pinned by a test that drives a panicking read through the
refresher and then asks it to read again.

## Read-only, everywhere

The boards write nothing: the operator-API reads are GETs, the state files
are read with `read_to_string` on a reader thread under a bound, and the
tower's `git` calls are `symbolic-ref`, `log`, `rev-list`, and
`status --porcelain` — all inspection. The ahead/behind comparison is
`rev-list --left-right --count HEAD...@{upstream}`, a local comparison that
never fetches. A test pins the contract by fingerprinting the scratch
sources before and after a full read of all three boards.

## What is not built

- **The browser face.** `dsh-board-seam`'s guarded HTTP route and the
  fork's React board pages are not ported; the TUI pane is this slice's
  surface, and the reads are the part a later web surface would reuse.
- **Workflow runs and agents.** The fork's router board also reads
  `/api/workflow-runs`; this board reads state, usage, and suggest, which
  is what the kit on this machine answers.
- **A scroll cursor on the tower.** A scan of thirty repos shows as many
  cards as the pane holds and names the rest (` +N more`); the pane is a
  glance surface, and the projects are ordered dirtiest-first so the rows
  it does hold are the ones worth seeing.
