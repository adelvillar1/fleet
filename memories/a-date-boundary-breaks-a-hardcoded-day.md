---
name: a-date-boundary-breaks-a-hardcoded-day
description: "feedback 2026-11-07 — a test that asserts a date-stamped artifact against a hardcoded day fails silently at the UTC rollover; the retire/supersede path stamps valid-until from calib::today() (real UTC), so the assertion must follow the real day, not the date the test passes to the function under test"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

`the_measured_set_separates_known_wrong_from_known_right` (src/memory.rs)
passed a fixed `"2026-11-07"` to `dream()` and then asserted the retired fact
was `forgotten("2026-11-07")`. It failed at the 2026-11-08 UTC rollover: the
retire path (`Memory::supersede`) stamps `valid-until` from `calib::today()`,
which is the REAL UTC day, not the date the test handed to `dream`. The test
went red on master too, with no change of mine — a load-sensitive test in the
product's own suite ([[a-load-sensitive-test-is-a-defect-in-the-code]]).

**Why:** a date-stamped artifact's meaning is relative to the day it was
written, and the writer reads the clock. A test that hardcodes the day it
thinks the writer used is a second, stale source of truth — the same shape as
a vendor-key list or a hand-kept table.

**How to apply:** when a test asserts on a date-stamped artifact, derive the
expected day from the same clock the writer reads (`calib::today()`), or
freeze the clock the writer sees. Never hardcode a day in both the call and
the assertion. And when a suite goes red at a date boundary, check the date
first — it is the cheapest explanation and the one that hides in plain sight.
