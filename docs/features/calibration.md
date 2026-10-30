# The calibration stream (slice 6)

Fleet's audit spine: one `events.jsonl` per UTC day, in `devdec`'s row
vocabulary, written through a bounded non-blocking queue and read back as
evidence. Every routing decision, every tool-call screening, and every run
ending lands here — graded, not just recorded.

The contract is dev-decisions' own: it **gates the operations, grades the
outcomes, and earns autonomy on evidence**. Fleet's rows land in the file
devdec itself writes, so the calibration dataset is one stream rather than
two, and `dev-decisions log` reads Fleet's rows beside its own.

```
src/calib.rs     the vocabulary, the writer, the reader, the surface
src/route.rs     calls the stream; owns <FLEET_HOME>/config.json
src/agent.rs     records the screening and turn rows
```

## Where the stream lives

The root is resolved in this order (`calib::resolve_root`):

1. **`<FLEET_HOME>/config.json`'s `calib.log_root`** — an explicit
   override, for a machine that keeps its dev-decisions tree elsewhere.
   A misspelled key is rejected the way a provider's is.
2. **dev-decisions' own convention** — `<user home>/.local/share/dev-decisions/logs`.
   This is the default, because the point of the stream is that the
   external `devdec` CLI reads Fleet's rows beside its own.
3. **`<FLEET_HOME>/logs`** — the fallback when no user home is known, so
   the root always resolves.

**dev-decisions names its root with neither an env var nor a config key.**
It pins the literal `LOG_DIR = Path.home() / ".local" / "share" /
"dev-decisions" / "logs"` in `scripts/devdec/config.py` and reads and
writes `<LOG_DIR>/<YYYY>/<MM>/<DD>/events.jsonl` from there. Verified by
grepping the package for `LOG_DIR`, `os.environ`, and the config
`DEFAULTS`: the only overridable paths in the whole config surface are
`providers.*`, and the CLI's `--log-dir` flag is a hidden argument on
`semantic-index` alone. Fleet therefore reproduces the same literal rather
than inventing a second home for its rows — the earlier `<FLEET_HOME>/logs`
placement (slice 5) is gone, and with it the reason devdec could not see
Fleet's decisions.

A root that cannot be created is not a resolution failure. The writer
counts the append failure and the run continues.

## The vocabulary

Rows are flat JSON objects tagged by `op`, exactly as devdec's are. The
field names are devdec's (`ts`, `provider`, `providers_used`, `verdict`,
`input_chars`, `input_sha256`-style hash, `latency_ms`, …), and
`input_hash` is the 16 hex characters of SHA-256 — the same algorithm and
the same truncation dev-decisions joins on (`hashlib.sha256(x).hexdigest()[:16]`),
hand-rolled in `calib::sha256` because the crate carries no crypto
dependency and needs exactly one primitive. Fleet names this field
`input_hash` where dev-decisions' own rows name the same value
`input_sha256`; there is deliberately no alias, because devdec joins on
the hash value (the 16 hex characters) and never on the field name.

Three ops are emitted today:

| op | one row per | fields |
|---|---|---|
| `route` | session's routing decision | `ts`, `provider`, `how` (judged/fallback/pinned), `workload`, `verdict`, `reason`, `model`, `providers_used`, `attempts[]` (provider/outcome/class), `input_chars`, `input_hash`, `latency_ms` |
| `tool_screen` | approval decision | `ts`, `tool` (namespaced for a lane call), `verdict` (approved/denied/always), `lane` (MCP calls), `input_chars`, `input_hash` (of the arguments) |
| `turn` | run ending | `ts`, `outcome` (ok/failed), `stop_reason`, `turns`, `usage`, `input_chars`, `input_hash` (of the task), `reason` (the failure class) |

`turn`'s `usage.total_tokens` is always 0: the streamed usage chunk carries
prompt and completion tokens only, and the agent's usage accounting sums
what the stream delivered. Prompt and completion are real; the total is
derivable as their sum and left at zero rather than fabricated (KI-019's
streamed-path family).

The session-log vocabulary is untouched: `turn`'s `stop_reason` carries the
session log's own endings verbatim — `completed`, `max-turns` from its
`end` record, and `interrupted` for the documented log-without-`end`
shape. A failure that reached the loop's own end is `failed`, with the
router's failure class in `reason`. An I/O failure is `interrupted`,
because that is what leaves the log without its `end` record. A signal
death (Ctrl-C, SIGTERM) also leaves the log without its `end`, but it
records no `turn` row at all — the crate installs no signal handler, so
`interrupted` is reachable today only through an I/O failure, not a kill.

`eval_corpus` is reserved for slice 13's scores. The name is claimed in
`calib::OP_EVAL_CORPUS`; nothing emits it yet, and the reader and the
surface need no format change when it lands.

**No task text and no key ever rides in a row.** A row carries a name, a
hash, and a count. `route`'s `input_hash` hashes the task; `tool_screen`'s
hashes the call's arguments; both keep the length beside the hash.

## The writer

One background thread per stream, fed by a bounded `sync_channel` of
1024 rows:

- **`record` is `try_send`.** Enqueue never blocks and never fails the
  caller. A full queue drops the row, increments the dropped counter, and
  logs one line. A decision is never blocked, delayed, or failed by its
  own audit trail — that is the whole reason the stream sits behind a
  queue.
- **The thread serializes appends.** One `write_all` per row under
  `O_APPEND`, so two fleet processes appending side by side cannot
  interleave into a corrupt line.
- **A write failure logs, counts, and continues.** A full disk or a
  permissions problem is a counted `failed`, one line on stderr, and the
  run proceeds.
- **A row lands in the day its own stamp names.** The writer thread may
  get to a row after midnight; the row's `ts` decides the directory, not
  the writer's clock.
- **The last handle's drop drains the queue**, bounded at two seconds. A
  row is never lost to a process that is on its way out, and a wedged
  writer cannot hold a shutdown open.
- **The counters persist** to `<root>/fleet-stream-state.json` (a `.json`
  name, invisible to devdec's own `rglob("events.jsonl")` and
  `rglob("*.jsonl")` readers), so a later `fleet calib` can show what the
  stream dropped. Two writers on one machine are last-writer-wins on that
  file; it is telemetry, and the rows themselves are the audit record.

## The first reader — bounded demotion

Slice 5's last acceptance criterion was "a graded failure moves the failed
provider down its chain". At route-prepare time, the stream is read back:

- **Only today's file**, and only the last `READ_CAP` (512) rows. That
  file is shared by every fleet process on the machine, so cross-process
  demotion is intended: another process's failures demote this session's
  walk.
- A candidate with **3 or more consecutive failed attempts**
  (`DEMOTION_THRESHOLD`) in those rows is demoted to the **end of this
  session's failover walk**. A `served` attempt resets the streak; a
  `skipped` one leaves it alone.
- **The sys1-judged selection is untouched.** Demotion reorders the walk
  only: the selected candidate keeps the head, every other candidate keeps
  its declared order, and the demoted ones move to the end. A candidate
  that is both selected and demoted stays at the head.
- The demotion rides in the decision row's `reason`
  (`; demoted deepseek (3 consecutive failures today)`), so a later
  reader sees why the walk was ordered as it was instead of inferring it.
- An unreadable or absent stream is a no-op. "No evidence" is never an
  error, and never a demotion.

## `fleet calib`

Today's rows as cards — one per op with its counts, the writer's
counters, and the last few rows — in the card style of `fleet lanes` and
`fleet providers`. `--day YYYY-MM-DD` shows a past day. No network, no
lane spawn, and no writer of its own: this is a read of the audit trail,
and a day with no rows says so rather than failing. The exit code is
always success — the stream is an artifact, not a gate.

**The face's rail (2026-10-30).** The calibration surface is the work
rail's idiom, the knowledge group's fifth: **the gate** leads — the
failover state, the fit rule in its own numbers, and the fit cells as
one-line rows whose press opens the pane card (counts, weighted rates,
spend, ask-sys1, tabs stood down) — then one section per op the day
carries, in the substrate's own order, badge = its row count. The
sections carry the ops the face used to drop: `delegation` (the child
session and its branch), `verify`, `embed_timing` (KI-249's
measurements), `lifecycle` — the stream's `OPS` now answers all
thirteen. The **eval runs** section renders the last run's twenty
scenarios as one-line rows, pass badges, failures named. The **day is at
hand**: the instrument steps back and forward, `#/calibration/<op>?day=`
addresses a day directly, and an empty day renders its honest empty
state. A row's press opens its record in the pane.

**No new TUI surface this slice.** The stream is an audit artifact; the
TUI's lane and provider surfaces already carry the live state a user
watches, and a per-row stream view belongs with the dashboards that
consume it, not with the agent's own pane.

## Reading it with devdec

The rows are in devdec's file, in devdec's vocabulary, so the external CLI
reads them with no adapter:

```sh
dev-decisions log --tail 5                       # text, beside devdec's own rows
dev-decisions log --format json                  # the full rows
```

Verified on this machine (2026-10-13): two real `fleet run` invocations
landed 5 rows in `~/.local/share/dev-decisions/logs/2026/10/13/events.jsonl`
beside 1313 of devdec's own, and `dev-decisions log` printed them in both
formats with exit 0.

The "a full disk never fails a decision" half was verified the same way:
with the day's file made unwritable, a real run logged
`the calibration append failed (best-effort, the run is unaffected):
Permission denied (os error 13)` — once per row — and still completed with
exit 0 and its answer on stdout. The counters' own file failing is named
once and nothing else changes.
