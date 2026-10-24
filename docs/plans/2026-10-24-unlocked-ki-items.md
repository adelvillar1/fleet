---
status: completed
created: 2026-10-24
updated: 2026-10-24
slug: unlocked-ki-items
---

# Plan: the unlocked KI items — the mechanical backlog

## Context

The owner-input audit emptied the waiting-on-you set, and the owner
called to proceed with what remains: nine open rows, all mechanical.
One of them (KI-190) turned out to have been fixed nine days ago —
`e1796da` landed the content-keyed lane entries, the walked-confidence
pin, and the beat-owned third-vector cell, and the row never closed —
so it closes on evidence. The rest are real work, each bounded.

Working discipline: one gate, one commit per increment, explicit paths;
each ledger row updated in the same commit as its work.

## Approach

The increments and the criteria they carry: increment 0 closes the
already-fixed row on evidence (C0); increment 1 opens the PowerPoint
door (C1); increment 2 writes the headless fragment beside the session
(C2); increment 3 words the intent skill's guidance (C3); increment 4
drives the partial-embed shape (C4); increment 5 writes the smoke guard
into the orientation doc (C5); increment 6 names the missing variable
in blend1's refusal (C6); increment 7 lands the embed-duration
measurement (C7); the close-out runs the full gate and reconciles the
ledger, carrying C8.

### Increment 0 — the evidence close (KI-190)

`e1796da` (2026-10-15) carries everything the row asked: content-keyed
lane entries keyed on each route row's `input hash`, the
walked-confidence pin (a regression writing 0.0 fails the check), and
the third-vector cell read from the beat's own rows — the day file no
longer truncated by a fixture. The remaining direct fixtures (the
ride-back split, the unfit cell, the flat sample) carry the by-design
comment: the beat's own batch cannot build a flat sample any more. The
row closes naming the commit.

### Increment 1 — the deck's PowerPoint door (KI-239)

`deck::to_pptx` exists and is tested; nothing can call it. The `deck`
tool gains `format: "pptx"`: `run_deck_with_event` writes the `.pptx`
beside the render and names the path in the summary the model reads.
Pinned: a test that asks for pptx and finds a valid zip at the named
path.

### Increment 2 — the headless deck fragment (KI-241)

Headless, the deck exists nowhere the owner can open. The deck run
writes the fragment beside the session (`deck-<stamp>.html`) and the
summary names the path — in every surface, with the face's `Work::Deck`
event unchanged beside it. Pinned: a headless run leaves the file.

### Increment 3 — the intent skill's guidance (KI-240)

The deck intent skill's build step says the deck tool's schema is
authoritative and the source need not be read — the 25-turn research
spiral started with reading `deck.rs` for a schema the declaration
carries. A wording change in the skill, plus the row closed.

### Increment 4 — the partial-embed shape (KI-268)

The /api/rank route test admits embedder-present and embedder-absent;
the route's degradation also allows a partial embed. The test harness
drives the embedder double so some candidates answer and others do not,
and the reply's shape is asserted as the route actually ranks it.

### Increment 5 — the smoke guard, written where the smoke is (KI-066)

The smoke pattern is a session-side artifact; the guard becomes a
standing sentence in `docs/harness-orientation.md`'s environment
section — before any unattended `--yes` run, assert the mock's port
file (or pin the candidate) and fail closed otherwise; a pinned
candidate alone has already proven insufficient.

### Increment 6 — the named-env refusal for blend1's refresh (C6, KI-310)

blend1's adapter catches the sibling import and returns the missing
variable by name — "refresh needs the sem1 core: set
BLEND1_SEM1_PYTHONPATH to services/sem1/src" — and the `refresh` tool's
description says it needs the sibling wiring (unlike ask/doctor/search).
C6 is this increment's deliverable, landed upstream first (the merged
main, where the adapter now lives), then fleet's vendored copy synced
to it, pinned by a test that launches the server with only blend1's own
path and reads the named variable back.

### Increment 7 — the embed-duration measurement (KI-249)

The honest fix measures before extending: embed calls record their
duration (the calibration stream's latency vocabulary, redacted) so a
day of real use accumulates the tail the bounds should come from. The
bounds themselves do not move this session; the row stays open naming
the instrumentation as landed.

## Acceptance criteria

- [x] **C0 — KI-190.** The row reads closed naming `e1796da` and the
      walked-confidence pin; no code in that commit.
- [x] **C1 — KI-239.** The `deck` tool accepts `format: "pptx"`, writes
      a valid `.pptx` beside the render, and names the path in the
      summary; by test.
- [x] **C2 — KI-241.** A deck run writes `deck-<stamp>.html` beside the
      session and the summary names the path, on every surface; by
      test.
- [x] **C3 — KI-240.** The deck intent skill's build step says the
      tool's schema is authoritative; the row closes.
- [x] **C4 — KI-268.** The rank-route test drives a partial embed and
      asserts the shape the route actually permits; by test.
- [x] **C5 — KI-066.** The smoke guard is a standing sentence in
      `docs/harness-orientation.md`; the row closes on it.
- [x] **C6 — KI-310.** blend1's `refresh` names the missing variable in
      its refusal and its description; upstream and fleet's vendored
      copy agree; by test.
- [x] **C7 — KI-249.** Embed durations land in the redacted
      calibration/log vocabulary; the row stays open naming the
      instrumentation and the day of data it waits for.
- [x] **C8 — close-out.** The full gate green by exit code; the ledger
      agrees with the tree; KI-252's gated moves and KI-249's
      bound-change remain the named out-of-scope.

## Files to be touched

`src/eval.rs` (nothing — evidence only); `src/tools.rs`; `src/deck.rs`;
`src/gui/server.rs`; `src/memory.rs`; `src/gui/bridge.rs` (rank double);
`docs/harness-orientation.md`; `docs/known-issues.md`; the deck intent
skill; `services/blend1/service/mcp_server.py` (upstream + vendored);
the tests beside each site.

## Out of scope

- KI-252's moves 2 and 3 (the Matryoshka pre-filter, the asset
  library) — blueprint slices gated on their own measurements, each
  wanting its own plan.
- Moving any timeout bound (KI-249's second half) before a day of
  measured durations exists.

## Verification

The gate's four commands per increment where Rust changes; the full
gate at close-out, exit codes read directly. blend1's suite for its
increment. Ledger rows in the same commits as their work.

## Linked artifacts

- `docs/known-issues.md` — the rows
- `docs/plans/2026-10-14-fleet-calibration-loop.md` — the fit leg's
  original design, now landed
