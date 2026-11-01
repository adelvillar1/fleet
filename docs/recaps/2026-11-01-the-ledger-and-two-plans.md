# The ledger, and two plans walked to completion

**2026-11-01** · the day's second session (the design-parity session
ran beside this one in the same worktree for part of it). The ask was
one line — *"let's work on the known issues ledger"* — and it became:
a triage, two plans drafted and ruled, and both walked green. The
ledger went from **38 open to 7** in one session.

## The arc

1. **The warmup's find**: `ki_ledger.py` was red — eight problems,
   all from four rows the KI-385 sweep had filed with their evidence
   merged into the issue cell (five cells where the header declares
   six). Repaired by splitting each at its evidence boundary
   (`e7a0a21`).
2. **The triage**: the 38 open rows grouped into four clusters —
   ledger tooling (B), five pin-sized defects (A), the corpus-refresh
   family (~15 rows, C), and the owner-decision rows (D).
3. **B and A closed**: the checker learned both vocabularies —
   status (`d62c217`, KI-322) and severity (`16600fe`, KI-391, with
   all 30 drifted cells normalized to the six) — and `fleet ki append`
   files a row under the ledger's own lock (`4e12b01`, KI-331); the
   five A defects closed with pins (`be4488a`): the sweep's double
   row, the signless age, the trial reason, the typed silent-turn
   record, the guard's over-fire.
4. **Two plans drafted, one revised twice**: the owner first pushed
   back on my O7 deferral (*"why defer ki-369?"* — the bite count was
   already paid: KI-221/224, every 10-28 catch-up, KI-387's four
   mechanical failures), then, asked *"is what you are suggesting the
   right fix or the easy fix?"*, forced a re-audit that flipped O2,
   O3, and hardened O5.
5. **The D plan — seven rulings, seven follow-throughs, complete**
   (`cd39eff`): the relay join gate (a059ce0), the send cap in
   product words (9e14ddf), the six-word severity taxonomy
   (16600fe), the no-machine-liveness class rule (f99265f), the
   substance-keyed gitignore with the writer's stamp (5449324), the
   write-through recall index (5a041e8), and the one-command lane
   route (2cda8e4 + 2666418).
6. **The C plan — five workstreams, eighteen rows, complete**
   (`0871e25`): the embed wedge (371421a), the source-set contract
   (bb0f61f), the job runtime (5777443), the disk promises (545c630),
   and the driver edges (103033c).

## Commits (master)

| Hash | What |
|---|---|
| `e7a0a21` | the four malformed rows repaired |
| `d62c217` | the checker refuses an unknown status (KI-322) |
| `4e12b01` | `fleet ki append` files under the lock (KI-331) |
| `be4488a` | five pin-sized defects (KI-384/383/382/352/353) |
| `56438be` | the two plans drafted |
| `a2ed33c` | O7 revised on the owner's push — build it |
| `25723a9` | the re-audit: right fix vs easy fix |
| `a059ce0`…`b3ceeb0` | O1–O6 landed; `2cda8e4`+`2666418` O7; `cd39eff` D complete |
| `371421a`…`0871e25` | W1–W5 landed; C complete |

## What is proven

- **The full gate, green at every commit**: fmt, clippy, build, and
  test — the final close-out run read **1,907 passed / 0 failed**
  across every target, plus the four vendored-service pin files each
  run directly under python3 (sem1's error split, blend1's budget,
  the source set, the server contract).
- **The nightly chain, live**: `fleet mcp call blend1 refresh` ran a
  real pmocockpit refresh end to end — the verb, the lane spawn, the
  cold window, the reply — and the reply carried every W2 field in
  production (`empty_wildcards` naming the project's satisfiable root
  glob, both deltas empty, `action: unchanged`). The 14 reconciled
  jobs carry the same route for their next fire.
- **The ledger itself**: 392 rows, **7 open**, checker green — the
  remainder are the embed-bounds plan's two window rows and five
  class-record rows, all correctly elsewhere.

## The traps this session hit

- **A cell-boundary script anchored at the wrong cell.** Closing
  five rows with a status-replacement script, I sliced from the
  id-prefix position — the severity cell's start — and mangled all
  five rows into five-cell ghosts. Recovered from HEAD and redone
  with the checker's own `split_cells` rebuild; the lesson is now in
  the method: parse with the checker's parser, rebuild the line from
  its cells, never hand-slice markdown tables.
- **A notification consumes no mock entry.** The `fleet mcp call`
  pin hung for 700 seconds (the call budget) because my scripted
  mock gave the `notifications/initialized` line an entry — mock-mcp
  skips id-less lines *before* popping — so the call's reply never
  came. Two entries, not three; the script lesson is pinned beside
  the test.
- **The id counter is shared**: without a `tools/list` in the flow,
  the call's id is 2, not 3 — my first script answered 3.
- **`days_between`'s sign is inverted from intuition**: the correct
  pair's difference is *negative* (`earlier` minus `later`), so my
  first KI-383 guard (`>= 0`) refused every correct order and the
  existing pins caught it in one run.
- **The shared worktree's branch hazards, twice**: two of my commits
  landed on the parallel session's `design-parity` branch (I checked
  HEAD but did not *gate* on it), and their K1-K6 commit swept my
  uncommitted `cron.rs` edit into their history — O7's task-string
  rewrite had to be re-applied on master. The split that fixed it: a
  **second worktree** (`fleet-master-wt`) for all my commits, their
  branch left to itself. Their mid-flight commits also broke master's
  test build twice (a `Record::Memory.design` pattern, a clippy
  lint) — both folded with credit.
- **The quote exemption was too broad**: "every quoted occurrence is
  a path" killed the deliberate string-literal identifier channel
  (`let _ = "judgeRoute"` must refuse) — the pin that guarded that
  design caught my first cut, and the rule now requires a
  path-composition context.
- **A timeout is not an absence**: the sem1 split (KI-346/354) is
  the day's thesis in code — `socket.timeout` was an OSError, so a
  live-but-queued server was told to start itself.

## Open

- **The relay join gate's deployment is the owner's hand**: set
  `RELAY_JOIN_SECRET` on both OpenShip services and the two
  `FLEET_*` vars in `.env`, then redeploy — until then the relay
  dial refuses at launch, which is the gate working (named in
  KI-367's row and `docs/features/remote-access.md`).
- **Seven rows open**, each elsewhere by design: KI-249/KI-252 (the
  embed-bounds plan's window), KI-317/320/323/326/327 (class-record
  rows whose instances are fixed).
- **The queue-depth health tool** (KI-325's third ask) and the
  embedder-more-slots alternative (KI-344's) are the named
  follow-ons, recorded in their rows.
- **KI-175** (the CLI's missing `--phrases` flag) and **KI-196**
  stand as their own rows beside the closed KI-357.

## Files changed

- **Ledger tooling**: `scripts/ki_ledger.py` (both vocabularies),
  `src/doctor.rs` + `src/cli.rs` + `src/main.rs` (`fleet ki
  append`), `docs/known-issues.md` (25 rows closed today).
- **The five A defects**: `src/lifecycle.rs`, `src/main.rs`,
  `src/agent.rs` (the typed turn-failure record + the record's
  bookkeeping arms), `src/session.rs`, `src/gui/bridge.rs`,
  `src/tui/app.rs`, `src/nomenclature.rs`.
- **D's seven**: `relay/relay.py` + `src/remote/relay.rs` +
  `src/gui/mod.rs` (the join gate), `src/gui/server.rs` (the product
  words), `src/tools.rs` + `src/memory.rs` (the write-through),
  `.gitignore` + `src/cron.rs` (the stamp), `src/mcp_registry.rs`
  (the lane-call route).
- **C's five workstreams**: `services/sem1/…/http_llama.py`,
  `services/blend1/…/corpus.py`, `services/blend1/service/
  mcp_server.py`, `src/cron.rs`, `src/lanes.rs`, `src/processes.rs`,
  `src/agent.rs`, `src/nomenclature.rs`, and four new pin files
  under `services/*/tests/`.
- **Docs**: both plans (complete), this recap, the two contract
  docs' verb rows, `docs/features/remote-access.md`'s threat model
  and deployment section.

## Notes

- **The owner's two questions were the session's hinge**: "why
  defer ki-369?" turned a wrong recommendation into the lane-call
  verb; "right fix or easy fix?" turned three of my seven
  recommendations. Both are recorded in the open-calls plan's
  history, and the re-audit's flipped calls are the ones I would
  have shipped wrong.
- **The parallel session is building design parity in the same
  repo**: their K1-K6/K2 commits, the centered-composer face work
  (uncommitted, reddening the face target in the shared tree), and
  the `Record::Memory.design` field all interleaved with this
  session's. The worktree split is the standing answer; the folds
  are credited in the commits that carried them.
- **Sweeps**: the cache sweep runs at the recap step (daily
  maintenance covers the rest); the ledger checker reads 392 rows, 7
  open, zero problems.
