---
name: vendor-sync-deletes-carried-files
description: "Re-vendoring a lane with rsync --delete deletes fleet-side carried files (perception.py, the adapter's perceive tool) — the lane boots, tests hang instead of failing, and expect_err masks the early error"
type: project
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

2026-10-24, the sys1 re-vendor (KI-029, commit 1543905): `rsync -a
--delete` from upstream `~/Projects/sys1/src/` deleted
`services/sys1/src/sys1/perception.py` (the documents session's
parse_document — a fleet-side carry that never landed upstream, the
same pattern KI-003 named) and the first pass also replaced the
fork-working-tree adapter, silently dropping the
`sys1_perceive_document` tool. The vendored copies are upstream + named
carries, never upstream alone.

**Why:** the failure was a HANG, not a red test. The lane booted
without the tool, `parse_via_lane` errored before any vendor HTTP
connection, `expect_err` passed vacuously on the early error, and the
test blocked forever at the served double's `join()` — waiting for
four phantom connections. A test that should fail, hanging instead,
cost an hour of orphaned cargo locks and stack samples before an
instrumented double showed zero connections arriving.

**How to apply:** (1) re-vendor with `rsync -a` WITHOUT `--delete`,
then diff deliberately; every file that exists only fleet-side is a
carry that `VENDOR.json`'s fleetVendor note must name (perception.py,
and the adapter's perceive-tool merge, both named now). (2) A test
hanging at a join/poll after an `expect_err`/`unwrap_err` means the
error came EARLY and the fixture is waiting for traffic that will
never arrive — instrument the fixture (count accepted connections)
before debugging the protocol. (3) Orphan hygiene: an interrupted
cargo leaves the test binary alive holding the target flock and test
ports — a later suite then wedges with no rustc running; `lsof
target/debug/.cargo-lock` and `ps aux | grep deps/fleet-` name the
holders, kill before diagnosing. Related: [[a-test-that-passes-on-an-idle-machine-has-not-been-tested]].
