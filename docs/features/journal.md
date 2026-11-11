# The event journal — the durable, total-ordered bedrock (armada L2)

Every state change the layers above care about lands in one durable home:
the security audit (L3), the communication bus (L5), the workspace events
(L7), and the control plane's cross-install tracing (L8). Before L2 the face
had only an in-memory broadcast (drops on lag) and a 4096-cap replay deque;
seats and queued runs died on crash, and the calib stream is deliberately
lossy. The journal is the durable, total-ordered record those layers ride.

It is a **responsible citizen** (D3): bounded by a retention budget, named in
every curtail, never a silent drop — deliberately unlike calib, which stays
advisory and untouched (D6). The session log stays the conversation record;
the journal is a new home, not a promotion of either.

## The shape (D10–D13)

- **Day-partitioned append-only JSONL** at `<home>/journal/YYYY/MM/DD.jsonl`,
  with a **global monotonic `Seq` (u64)** persisted at `<home>/journal/seq`,
  so order is total across days and survives restart.
- **`Event { seq, ts, kind, install, principal?, session?, payload }`** — the
  install-id (D7) rides every event; the principal (D8) and session ride when
  there is one.
- **Durability:** `O_APPEND` + flush per record (process-crash durable every
  record); fsync at a declared cadence (`DEFAULT_FSYNC_EVERY`, power-loss
  durable periodically). The counter is flushed per write, not synced — the
  hot path holds no per-event fsync.
- **Durable named consumers** persist a cursor at
  `<home>/journal/consumers/<name>.cursor` and replay from it, so a lagged or
  restarted consumer never loses an event.
- **Retention:** `curtail(max_age_days)` removes day-files older than the
  bound by name (default 30 days); the current day is never curtailed.

## The contract (for the layers above)

```text
Journal::open(home) -> Journal                 // reads the persisted seq + install-id
journal.append(kind, principal?, session?, payload) -> Seq
journal.replay(from: Seq) -> Vec<Event>        // total order, torn-tail tolerant
journal.cursor(consumer) -> Seq                // 0 for an unknown consumer
journal.advance(consumer, seq)                 // persist the cursor
journal.subscribe(consumer) -> Vec<Event>      // replay from the cursor
journal.curtail(max_age_days) -> Vec<String>   // named removals
```

The sequencer is re-read under its flock on every append, so concurrent
writers serialize and never reuse a number. See
`docs/plans/ARMADA-CONTRACTS.md` §3 and `ARMADA-DESIGN-DECISIONS.md` D10–D13.

## The bridge hook (D13)

`record()` — the one seam every UI event crosses — appends to the journal
**additively** beside the broadcast: the event is journaled, then broadcast
as before. The append happens outside the state lock (the hot path holds no
IO), and a journal failure is named, never breaks the stream. The
seat/control/finished events that died on crash now survive.

## Surfaces

- **`fleet journal`** prints the last sequence number, the recent trail, and
  each named consumer's cursor.

## Verification

`cargo test -p fleet --lib journal::` pins: strictly-increasing seq that
persists across restart, durability (a restart replays the event), consumer
resume (a restarted consumer resumes exactly where it stopped), total order
+ `from` filtering, named curtail (old day-files removed, today kept), and
the install-id on every event. The bridge test
`record_journals_the_event_additively_beside_the_broadcast` pins the additive
hook (the seat is journaled while the broadcast still fires). The face
harness confirms the stream is unchanged.