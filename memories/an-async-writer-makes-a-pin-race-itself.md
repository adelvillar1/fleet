---
name: an-async-writer-makes-a-pin-race-itself
description: "feedback 2026-11-01 — a new pin counted a queue-backed stream's rows immediately after writing and flaked once in the full suite (0 vs 1); calib's Stream::record is try_send onto a writer thread, so the pin must settle on pending() and fail loudly on dropped/failed, and a test helper that returns 0 on a read error dresses the race up as a missing row"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Closing KI-384 (an applied sweep disable recorded twice), the new pin
`an_applied_sweep_records_each_disable_once` passed in its module run and then
failed once under the full suite: `left: 0, right: 1`. The count was taken
immediately after `sweep(...)`, but `calib::Stream::record` is not a file write
— it `try_send`s onto a queue a background writer thread drains to
`<root>/<YYYY>/<MM>/<DD>/events.jsonl`. The rows were in flight, not missing.
The first form of the test helper also had `let Ok(entries) = read_dir(dir)
else { return 0 }` — a silent zero on any read error, which would have dressed
the same race up as "the row never landed."

**Why:** the stream's own test module already carries the pattern (`settle()`:
wait while `pending() > 0` against a deadline); a pin written in another module
has to rediscover it, and a flaky pin is a defect in the pin — the same
load-sensitivity law as [[a-load-sensitive-test-is-a-defect-in-the-code]].

**How to apply:** before pinning anything a queue or background thread writes,
find the drain API (`pending()`/`written()`/`dropped()`/`failed()`) and settle
on it; then assert `dropped() == 0 && failed() == 0` so a real write failure is
loud, and make every fixture read `expect`/`unwrap_or_else(panic)` instead of
defaulting to zero. If the flake still appears, the settle budget or the
silent-zero was the remaining lie.
