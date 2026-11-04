---
name: ki-405-probe-retry-pin-is-load-sensitive
description: "project 2026-11-02 — KI-405: the probe-retry pin in src/memory.rs fails under a loaded suite because its double refuses a fixed four CONNECTIONS while the probe's own retry budget is sized in ATTEMPTS; a try that fails before reaching the backend spends the budget"
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

Filed as **KI-405** on 2026-11-02 (ledger 404 rows / 13 open, checker clean).
Found by the full-suite run that followed the native shell landing —
`cargo test` exited 101 with one failure, then passed 2,002/0 on the immediate
re-run.

**The test:** `memory::tests::the_probe_retries_after_a_pause_so_a_flap_is_not_a_missing_backend`
(`src/memory.rs:6054-6112`). It stands up a double that refuses the **first four
connections** and answers within the second, then asserts the probe does not
read the flap as "no backend".

**Why it reds under load:** the double's own comment names the mechanism —
*"a try can fail before it ever reaches the backend, and the probe's own retry
count is the bound the double is sized against"*. Under a parallel suite a try
can fail before it ever reaches the backend, so the four-connection refusal
budget is spent on failures that were never the flap, and the probe returns
"no backend". The sizing is a fixed **connection count** while the thing being
tested is a retry budget in **attempts** — the observable and the fixture are
measured in different units.

**How to fix (the row's next action):** make the observable name itself — the
double should refuse a number of *attempts* rather than connections, or the
probe's retry budget should be a named constant the double is sized against the
way `PROBE_TIMEOUT`'s pin is (`src/memory.rs:6040-6051`, which asserts the
bound equals `embed_bound_ms(MAX_MS, MEDIAN_MS)`). Then prove it under load —
KI-385's four-CPU-burner discipline — rather than passing only when the machine
is quiet.

**The lesson this adds to [[a-load-sensitive-test-is-a-defect-in-the-code]]:**
that memory lists four shapes (a process-global switch, a blocking probe, a
socket closed with unread bytes, a budget as a latency contract). This is a
**fifth**: a fixture whose refusal is counted in a different unit than the
mechanism it stands in for. Before writing a double that "refuses the first N",
ask what the real thing counts — connections, attempts, or time.

Related: [[a-load-sensitive-test-is-a-defect-in-the-code]],
[[a-test-that-passes-on-an-idle-machine-has-not-been-tested]],
[[verification-depth-matches-the-change]].
