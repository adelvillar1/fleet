---
name: a-load-sensitive-test-is-a-defect-in-the-code
description: "A test that reds only under a parallel suite is naming real state in the product — a process-global switch, a blocking probe, a socket closed with unread bytes, or a budget that asserts latency"
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

KI-385 was filed as "the relay suite is load-sensitive" and sat open for a
day as a test-harness nuisance. It was four product defects, and every one
was invisible to a single-test run:

1. **A process-global switch.** `home::unlock_vault` holds a `OnceLock`.
   One test unlocked it, and every settings test running at that moment
   wrote its secret to the vault instead of the `.env` it asserts. A
   green-alone test reddening its neighbour is always this shape: look for
   `OnceLock`/`static mut`/`env::set_var`/a global `Mutex` in the code the
   failing test *and* its neighbours touch.
2. **A blocking probe behind a global initialiser.** `probed_embedder()`
   was `OnceLock::get_or_init(probe)` with a probe that retries for ~17 s.
   The first caller ran it on its own thread; inside the face's async
   worker that meant a stalled tokio worker, and two of them meant a
   server that answered nothing at all (a 30 s POST returned **0 bytes**).
   A failure was also cached for the process's life.
3. **A socket closed with unread bytes.** The fake relay answered without
   draining the request body; the kernel reset the connection under the
   client's header read — ureq's `Error encountered in a header: Invalid
   argument (os error 22)`. Every test server must read head **and**
   `Content-Length` body before replying, and half-close before dropping.
4. **A budget used as a latency contract.** `ask()`'s 3 s budget red
   routes that assemble the machine's whole library (452 skills, the
   settings catalog) on a busy machine with an empty reply.

**A fifth shape, added 2026-11-02 (KI-405).** A **fixture counted in a
different unit than the mechanism it stands in for.** The probe-retry pin's
double refuses a fixed four *connections* while the probe's retry budget is
sized in *attempts* — under load a try can fail before it reaches the backend,
so the budget is spent on failures that were never the flap and the probe
reads "no backend". Before writing a double that "refuses the first N", ask
what the real thing counts. See [[ki-405-probe-retry-pin-is-load-sensitive]].

**Why:** the suite is the only place these reach each other; in
production each is a stall or a silent degradation that nobody sees.

**How to apply:** when a test fails only in a full parallel run, do not
raise a retry — find which of the four it is. Ask: what process-global
state does this path read or write? Does the fixture close cleanly? Is the
budget a hang detector or a latency promise? Then make the *observable*
name itself (a `degraded` flag, a `budget_exhausted` flag, two sentences
for two absences) and make the test skip **loudly** on a machine fact,
never silently. Prove it the way this was proved: the module alone (8×),
the full suite sequentially (6×), then the full suite under four CPU
burners (5×) — the load is the only evidence that counts.
See [[verification-depth-matches-the-change]] and
[[one-backend-many-callers-is-a-load-amplifier]].
