---
status: active
created: 2026-11-01
updated: 2026-11-01
slug: corpus-refresh-contract
---

# Plan: the corpus-refresh contract — one root for the nightly family

## Context

Twenty of the ledger's open rows were filed by the scheduled
`corpus-refresh-*` runs and their catch-ups (2026-10-27 through
2026-11-01). Read together they are not twenty defects: they are five
missing contracts, each named by several rows at once.

1. **The embed pipeline wedges instead of degrading.** One local
   llama-server (`--parallel 1`) serves every refresh, a socket
   timeout is reported as an absent endpoint (KI-346, KI-354), the
   read path has no budget of its own (KI-347), and a catch-up batch
   spends each project's rebuild budget queueing behind the others
   (KI-325, KI-344).
2. **The source set has two declarations that disagree.** The lane's
   default set (`DEFAULT_ROOT_DOCS`) and `seed_task`'s glob name
   different files (KI-345); a default derivation resolves against the
   child's cwd and reports a truthful zero that hides its cause
   (KI-339); a glob matching nothing is counted as a dark source
   (KI-343); and the report never says WHICH sources it acted on
   (KI-348).
3. **The job's runtime is re-derived from the server source every
   time.** The transport framing, the ping shape, and the cold-import
   window live in code comments a hand-driver has to rediscover
   (KI-387); `mark_dispatched` leaves the previous run's outcome
   standing (KI-363); the process surface cannot tell exited from
   unreaped (KI-358); and the lane's interpreter pin is conventional,
   not structural (KI-324).
4. **A run's disk promises are not one fact.** The supersede fold
   retires one node of a chain and leaves ancestors live (KI-355); a
   retirement note promised a replacement file that never existed
   (KI-356).
5. **The guard's edges bite exactly the driver a run writes.** A
   composed path segment is refused where the same literal passes
   (KI-330); a language keyword establishes a term that then refuses
   the keyword (KI-333); the `remember` bound is discoverable only by
   paying a refusal turn (KI-357).

This plan is the contract those five gaps close under. Each workstream
is independently landable; the order is wedge → runtime → sources →
promises → edges, because the wedge is what makes a nightly run hang
tonight.

## Approach

**Fix each contract where its rows say it lives; every fix lands with
the test that names it, and every row closes with that pin.**

### W1 — the embed pipeline degrades instead of wedging

- **KI-346 / KI-354 (C1)** — `sem1`'s embed arm splits by cause:
  `ConnectionRefusedError` keeps `unreachable … Start it`, while a
  `socket.timeout` answers `endpoint did not answer within {timeout}s
  — it is up but busy; retry`. Pinned against a stub that times out
  while alive.
- **KI-347 (C2)** — `corpus.search`'s query embed carries the budget
  the rebuild has (`EMBED_TIMEOUT_SECONDS` or a query-sized bound of
  its own); a slow endpoint still answers a single-query search. Pinned
  by the slow-endpoint test.
- **KI-325 (C3)** — refreshes serialize fleet-side (a per-project
  lock) or the driver carries an embed-call timeout with bounded
  backoff-retry, so saturation degrades to retry-later instead of a
  hung child; the lane-side queue-depth health signal (the row's third
  ask) is named as the follow-on, not this increment's gate.
- **KI-344 (C4)** — a catch-up batch serializes its refreshes (or the
  embedder gains slots — KI-162's suggestion), so one project's budget
  cannot be spent waiting behind five others; whichever lands, the
  row's wording records the choice.

### W2 — one source-set contract

- **KI-339 (C5)** — a refresh report names the cwd its default set
  resolved from, and an empty default derivation is refused by name
  rather than reported as a quiet `no-source`.
- **KI-343 (C6)** — the report splits satisfiable declarations from
  dark ones: a glob matching nothing is satisfiable, a named path
  matching nothing is dark, and one list for both is what made this
  row unavoidable.
- **KI-345 (C7)** — one declaration serves both surfaces: the lane's
  default set grows `README.md` (the job convention already says every
  root `.md` belongs), or `seed_task` passes the default set's three
  names — the divergence retired from one side, named in the row.
- **KI-348 (C8)** — the refresh report carries the source-set delta it
  acted on — the names added and removed since the stored manifest —
  so a definition change reads as itself instead of as a larger
  `rebuilt`.

### W3 — the scheduled job's runtime is named and honest

- **KI-387 (C9)** — the spawn contract is written where the next
  driver reads it: newline-delimited JSON-RPC (no Content-Length
  framing), `ping` answered `{}`, initialize replying only after the
  cold import (allow a 300 s window) — beside the launch spec in
  `src/lanes.rs` and in `mcp_server.py`'s transport docstring; and
  `respond()`/`error()` downgrade a broken pipe to a named warn-exit
  instead of a traceback on a client that hung up.
- **KI-363 (C10)** — `mark_dispatched` stamps `last_outcome` as
  dispatched (or clears it), so the store never claims an outcome the
  newest window has not produced.
- **KI-358 (C11)** — the process surface reads wait(2)/`ps stat` for
  the zombie state, so an exited child reads as
  finished-with-exit-code rather than still-running while unreaped.
- **KI-324 (C12)** — the lane pins its interpreter structurally: the
  launch spec ships a venv, or `mcp_server.py` refuses startup when
  `sys.version_info < (3,10)` naming the version and the
  `zip(strict=)` sites.

### W4 — a run's disk promises are one fact

- **KI-355 (C13)** — the supersede fold walks the chain: a supersede
  that retires fact A, whose `superseded-by` names B, whose names C,
  retires the whole walked set into the replacement — or, at minimum,
  writes its retirement note into every body it leaves in the chain.
- **KI-356 (C14)** — the retirement note and the replacement file are
  one disk fact, verified by a read-back before the run reports done —
  the run that promised `written-this-run` and wrote nothing cannot
  report success again.

### W5 — the edges that bite a driver

- **KI-330 (C15)** — `is_unrenamable` keys on the token, not on its
  occurrence's shape: a path segment under the owner's mandated home
  directory is a path, however it was spelled — so
  `os.path.join(user_home, "Projects", "fleet")` is writable beside
  the absolute literal.
- **KI-333 (C16)** — the vocabulary never adopts a term from a
  language keyword's own occurrence: Python `True`/`False`/`None` and
  Rust `true`/`false` are syntax, not nomenclature. (KI-353's all-caps
  exemption already covers the ambient names — `HOME`, `PATH`,
  `LANG`, `TMPDIR`; this is the keyword half.)
- **KI-357 (C17)** — the `remember` tool declares `MAX_PHRASES` in
  its description — the bound is instruction, discoverable before the
  refusal — and one policy governs both write paths.

## Acceptance criteria

- [ ] **W1 — the wedge degrades.** C1–C4 each land with their pin;
      a refresh under a saturated embedder retries with a named
      message instead of hanging or reporting an absent endpoint.
- [ ] **W2 — one source-set contract.** C5–C8 land; a definition
      change and a content change are distinguishable from the report
      alone, and the two declarations agree.
- [ ] **W3 — the runtime is named.** C9–C12 land; the transport
      contract is written where a driver reads it, the store never
      claims an unproduced outcome, and the process surface reads
      exit-vs-reap.
- [ ] **W4 — promises are one fact.** C13–C14 land; a chain fold
      leaves no live ancestor, and a run cannot report a write it did
      not read back.
- [ ] **W5 — the edges stop biting.** C15–C17 land with their pins.
- [ ] **Close-out.** Every row this plan names closes with its pin as
      evidence; the full gate green by exit code; the ledger checker
      green; the nightly jobs' next runs read clean against the
      contracts (named in the close).

## Files to be touched

`services/sem1/src/sem1/providers/http_llama.py` (C1),
`services/blend1/src/blend1/corpus.py` (C2, C5, C6, C8),
`src/cron.rs` (`seed_task`'s declaration, C7), `src/lanes.rs` (C9's
contract beside the launch spec), `src/cron.rs` (`mark_dispatched`,
C10), the process surface (C11), `mcp_server.py` (C9's docstring,
C12), `src/memory.rs` (C13, C14, C17), `src/nomenclature.rs` /
`src/agent.rs` (C15, C16), the driver lock or timeout (C3, C4),
`docs/known-issues.md` (the closes).

## Out of scope

- **KI-369** (the durable spawn route) and **KI-386** (the gitignore
  glob) — owner calls, planned beside this one in
  `2026-11-01-open-calls.md`.
- **KI-332** (remember's write-time promise) — an intent call, same
  companion plan.
- New features: the row that suggests a queue-depth health tool (the
  third ask of KI-325) is a follow-on, named but not gated here.

## Verification

The gate's four commands by exit code at close-out; each pin's own
run; a saturated-embedder rehearsal for W1 (a stub that delays,
proving retry-not-hang); the ledger checker over the real file as the
last act before the commit.

## Linked artifacts

- `docs/known-issues.md` — KI-324, KI-325, KI-330, KI-333, KI-339,
  KI-343, KI-344, KI-345, KI-346, KI-347, KI-348, KI-354, KI-355,
  KI-356, KI-357, KI-358, KI-363, KI-387
- `docs/plans/2026-11-01-open-calls.md` — the companion plan's owner
  calls (KI-369, KI-386, KI-332)
