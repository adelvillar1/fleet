---
status: draft
created: 2026-11-11
updated: 2026-11-11
slug: armada-l2-journal
---

# Plan: armada L2 — the event journal, the durable bedrock

## Context

The second bedrock. Today fleet has no durable, total-ordered event log: the
face's event stream is an in-memory broadcast (drops on lag), its "replay"
is a 4096-cap deque, seats and queued runs die on crash, and calib is
deliberately lossy. The security audit (L3), the communication bus (L5), the
workspace events (L7), and the control plane's cross-install tracing (L8)
all need a durable home that does not exist. This plan builds it: a
total-ordered, durable journal with named consumers — atom L2.1–L2.4 of the
decomposition. It is a responsible citizen (D3): bounded, named, settled.

## Design choices inherited (the ledger)

Conforms to **D0–D6** (shared) and **D7** (the install-id rides every event),
**D8** (the principal rides events that have one), **D9** (the principal is
the run's authority). Honors **D6** hard: calib stays advisory, the session
log stays the conversation record, JSONL before SQLite — the journal is a
**new** home, not a promotion of either.

## Decision-model check

Deterministic journal plumbing — append, sequence, persist cursors, curtail
by a budget. No per-item classify/gate/score decision; **no sys1 job fits**.
The retention curtail is a deterministic age/budget rule; the budget itself
learns from settles (D4) the way the resource contract's does.

## Approach

One new module, `src/journal.rs`: an append-only JSONL journal,
day-partitioned like calib but with a **global monotonic `Seq` (u64)** so
order is total across days. Each `Event` carries `{seq, ts, kind, install,
principal?, session?, payload}` — the install-id (D7) and principal (D8)
from L1. A named consumer persists its last-read `Seq` as a cursor and
replays from it, so a lagged or restarted consumer never loses an event.
`record()` in the bridge hooks the journal **additively** — every UI event
appends to the journal *and* broadcasts as today; the seat/control/queued-run
events that die on crash are now journaled. The journal declares a retention
budget and curtails old day-files by age, named — never a silent drop.

High-level steps:

1. `src/journal.rs`: `Event` + `Seq`; the sequencer persisted at
   `<home>/journal/seq` (atomic); `append()` (O_APPEND + flush per record,
   fsync at a declared cadence); day-partitioned files.
2. Durable named consumers: `<home>/journal/consumers/<name>.cursor`;
   `subscribe(consumer, from)` / `replay(from)`.
3. The retention budget + named curtail of old day-files (the responsible
   citizen, riding the footprint discipline).
4. `record()` (`src/gui/bridge.rs:12340`) appends to the journal additively;
   the broadcast stays. `fleet journal` reads the trail + consumer positions.

## Acceptance criteria

- [ ] C0: `journal.append` assigns a strictly increasing `Seq` that persists across restart; two events are totally ordered by `Seq` regardless of day file.
- [ ] C1: an appended event survives a process restart — `replay(from)` reads it back (durable, not the in-memory deque).
- [ ] C2: a named consumer's cursor persists; `subscribe(consumer, from)` replays from the cursor, and a restarted consumer resumes exactly where it stopped.
- [ ] C3: `record()` appends every UI event to the journal additively while the broadcast still works; a seat/control event is journaled (a test pins it).
- [ ] C4: the journal declares a retention budget and curtails day-files past it by age, naming the curtail (a test pins the bound and the name).
- [ ] C5: calib is unchanged (still advisory/lossy) and the session log unchanged; the four-leg gate passes and the face still streams (non-breaking).
- [ ] C6: `fleet journal` renders the event trail and a consumer's cursor position.

## Files to be touched

**Backend:**
- `src/journal.rs` — new: Event, Seq, append, consumers, replay, retention.
- `src/lib.rs` — `mod journal`.
- `src/gui/bridge.rs` — `record()` hooks the journal additively.
- `src/home.rs` — the journal path helpers.
- `src/cli.rs`, `src/main.rs` — the `fleet journal` verb.

**Tests:**
- `src/journal.rs` (module tests) — ordering, durability, cursor resume, curtail.

**Documentation:** see Linked artifacts.

## Out of scope

- The channel pub/sub bus and durable outbox (L5).
- The security audit rows (L3 writes them; L2 only provides the home).
- Migrating the SSE stream to read from the journal (L5).
- Promoting calib or merging the session log (forbidden by D6).
- Journal federation across installs (L8).

## Verification

**C0/C1/C2:** `cargo test -p fleet --lib journal` — ordering across day
files, restart durability, cursor resume.

**C3:** a bridge test records a seat event and asserts it lands in the
journal while the broadcast still fires.

**C4:** a test fills past the budget and asserts the named curtail.

**C5:** the four-leg gate; the face harness still streams.

**C6:** `fleet journal` renders the trail + a cursor.

## Linked artifacts

- `docs/features/journal.md` — new: the journal contract (Event, Seq, consumers, retention).
- `TECHNICAL-DOCUMENTATION.md` — the journal contract.
- `ARMADA-DESIGN-DECISIONS.md` — append D10–D13 (below).
- `../architecture/2026-11-11-armada-atomic-decomposition.md` — L2 atoms ticked when shipped.

## Risks

- **`record()` is hot (every token).** Journaling every event is write-heavy. Mitigation: flush per record + cadenced fsync (not per-token fsync); the retention budget bounds growth; coalescing the token stream is a measured tuning if the settles show need (D4), not a premature optimization.
- **Durability level** — flush is process-crash durable; fsync at turn boundaries adds power-loss durability. The cadence is a declared choice, named in the contract.
- **Ordering across processes** — the Seq sidecar is a single-writer counter under the flock; concurrent appenders serialize through it (the journal is one home, one writer thread, like calib).

## Dependencies

- `2026-11-11-armada-l1-identity.md` (the install-id + principal on events). Inherits ledger D0–D9.

## Design choices this layer settles (append to the ledger)

- **D10 — the journal is day-partitioned append-only JSONL at
  `<home>/journal/YYYY/MM/DD.jsonl`; `Event {seq, ts, kind, install, principal?,
  session?, payload}`; a global monotonic `Seq` (u64) persisted at
  `<home>/journal/seq`.** *Constrains:* L3 audit rows, L5 bus, L7 events, L8
  tracing — all ride this shape.
- **D11 — durability = O_APPEND + flush per record, fsync at a declared
  cadence; a retention budget curtails old day-files by age, named.**
  *Constrains:* every producer (bounded, never silent drop — unlike calib).
- **D12 — durable named consumers persist a cursor at
  `<home>/journal/consumers/<name>.cursor`; `subscribe(consumer, from)`
  replays from it.** *Constrains:* L5 channel consumers, L8 cross-install
  federation.
- **D13 — `record()` hooks the journal additively (append + broadcast);
  calib and the session log are untouched.** *Constrains:* L5 (the SSE stream
  becomes a journal projection later); the face keeps working unchanged.

## Notes

The journal is the home the rest of the stack settles into. It is additive
and non-breaking: the broadcast stays, calib stays advisory, the session log
stays the conversation record. Approve this and the ledger carries D10–D13
into L3 (security), whose audit lands here.