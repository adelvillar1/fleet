---
name: a-test-that-passes-on-an-idle-machine-has-not-been-tested
description: "feedback 2026-10-18/20 — four flakes in two sessions all passed alone and failed under load; two were timing races and two were test doubles that assumed one read brings the whole request. Reproduce with a build running beside the suite before calling anything a timeout"
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

A suite that is green run alone is not evidence that its tests are
sound. The 2026-10-20 port produced four failures that appeared **only**
inside the full suite, and every one had a nameable mechanism — none was
"machine load" in the sense of an excuse.

**Two were test doubles with the same framing bug.** The fake Telegram
server read until it saw `\r\n\r\n` and took whatever else was in the
buffer as the body. On an idle machine one `read` returns the head and
the body together, so the body was there; run beside a build, the
scheduler splits them, the body arrives empty, the send goes
unrecorded, and the test fails with `sent: []` — one run in three under
load, never once on its own. The relay's `answer_frame` had the
mirror-image defect: it rebuilt a request head from stored headers and
the relay had stripped `content-length`, so every POST body declared
zero bytes. Both are the same class: **a double that frames a request by
where the buffer happened to end rather than by the declared length.**

**The fifth, 2026-10-20: the same framing bug, mirrored by my own mock.**
The plugin tests' one-request TCP mock read once and answered at once, so on
the POST path the client was still writing the body when the socket closed;
ureq reported `Error encountered in a header: Invalid argument (os error 22)`,
which reads like a client bug and is not. The fix is the lesson: the mock must
read the request to its declared `Content-Length` before replying. A test
double that answers before the request is complete fails as a transport error
in code that is correct.

**Two were timing races.** A control-file test and a seat test asserted
after a fixed wait that was generous on an idle machine and marginal on
a busy one. Widening the bound (2s → 10s) fixed both; the bounded wait
still fails on a real deadlock, so nothing was lost. The sharper form of
this one, 2026-10-21 (KI-281): a bridge test polled the state document's
`session` *field* for a second and then asserted the `session` *event*
was in the event log — two different latches — so under load the poll
exited on the field while the broadcast queue still hadn't delivered the
event (the panic showed the message and skills events present and the
session event not). The fix is not a wider bound: **poll the exact
condition the assertion reads**, here the event's presence beside the id,
with a budget a loaded machine can meet.

**Why:** the failure arrives once in five runs, in a suite the owner
reads as "green", and the tempting response is to rerun until it passes
or to label it flaky and move on. Every one of these had a mechanism
that could be named, reproduced, and fixed — including the rank-route
test this file once recorded as "a real hole, not fixed": it has one now
(see the seventh).

**A sixth, 2026-10-21 — a deliberately slow path with a short read budget.**
The vault server test unlocked, initialized, then re-read `/api/settings`
through the test helper's fixed 3 s socket read; PBKDF2 at 210k
iterations under a full-suite load blew it. It passed alone every time and
failed inside `cargo test`. The fix is not a bigger global timeout: give the
expensive requests their own budget (`post_within` / `get_within`, 30 s) —
and make a shared test helper's budget an argument rather than a constant
when a new path can be legitimately slow. The same rule covers crypto
paths, subprocess round-trips (the lane probes), and any read that builds a
catalog.

**A seventh, 2026-10-21 (KI-280, corrected later the same day).** The rank
route's test failed in full-suite gates and passed alone. The FIRST
diagnosis (d78548d) — the helper truncating a slow body at its read
timeout — was a real weakness and the declared-length read stays as
hygiene, but it was **not this failure's mechanism**, and the ledger row
was corrected rather than quietly rewritten. The proof: in the failing
tree, line 2878 was the `starts_with("HTTP/1.1 200")` assert whose message
is the reply itself, and the reply was EMPTY — the head never arrived. The
route embeds over HTTP to the machine's shared llama-server (8901), which
the owner's live face keeps busy; under that load the reply took longer
than the helper's ordinary three-second budget and nothing came back. That
is the SIXTH instance's class, not the framing class: the fix is the rank
request's own 30-second budget (`post_within`), the remedy the vault's
PBKDF2 path already had. Lesson on top of the lesson: identify the failing
LINE IN THE TREE THAT FAILED (`git show <commit>:file`) before writing the
mechanism down — a plausible mechanism one line off is a wrong memory.

**A tenth, 2026-10-24 (KI-296's own test) — macOS discards received
data on RST.** The probe-retry test's TCP double refused N connections
by accepting and dropping them, then answered the next — green alone,
red in the full gate, twice, even after widening the probe's retries.
The mechanism: dropping a stream that still holds **unread request
bytes** sends RST, and on macOS an RST discards the reply bytes
already sitting in the client's receive queue — the answer was written
and then lost, and under load the server's drop beat the client's
read every time. The double must **read the request before
answering**, and the refused branches should drop *before* the client
writes (or accept the RST deliberately). A raw-socket double that
never reads is not a fast server; it is an RST cannon aimed at its own
client.

**How to apply:** reproduce with load, not more patience — run a
`cargo build` beside the suite, or a loop of the single test, until it
fails; a failure captured under load is worth more than ten passes
alone. When it fails, read the assertion before the shape: `sent: []`
and `EOF while parsing a value` both mean *the body never arrived*, not
*the operation was slow*. Then fix the framing — request or reply — to
the declared length and pin it; and when the reply is empty outright,
give the slow path its own budget instead of the shared constant.

**An eighth and ninth, 2026-10-22 (KI-284/KI-285): the family keeps
producing.** In one full-suite gate run on the approval-seat work:
`nomenclature::tests::the_proof_the_vocabulary_is_bounded_and_reports_itself`
(the vocabulary test's residual wall-clock sensitivity — the tree-fact
flag was split once already, and a grown suite moved the trip rate
again) and `remote::relay::tests::the_serve_loop_holds_one_poll_at_a_time`
(a fixed 1500 ms wait before asserting a counter). Both passed alone and
in the next full run. The same gate run also caught a *third* test
posting to the rank route on the ordinary 3-second helper
(`the_address_alone_opens_the_face_and_the_web_is_refused`) — the
KI-280 shape for the third time — which got its own `ask_within` 30 s
budget. The recurring tell: any test whose observable is a wall clock or
a shared backend will red under load no matter how green it is alone.

**A thirteenth, 2026-10-28 (KI-340) — a wall-clock poll the machine
outruns, fixed by waiting on the product's own signal.** The
leftover-control test polled the control file's removal for 15 s after
dispatching the run; under a fully parallel build the agent thread's
teardown outran it — red once, green alone and on the full re-run, no
regression anywhere near the diff. The fix is the family's real remedy,
not a wider bound: subscribe to the bridge's broadcast (the same channel
the faces read), break on `finished`/`failed` — the teardown runs before
either is emitted — or on the file's removal, with a 120 s deadline only
a hung run can reach. The bound a loaded machine could not meet is GONE
from the test, and the signal is the product's own seam rather than a
private one. Related:
[[a-flag-that-conflates-a-tree-fact-with-a-machine-fact]] (the same
bound-vs-load confusion one level up),
[[a-regression-test-must-fail-when-the-bug-returns]] (verify the
verifier),
[[a-verification-with-no-output-is-not-a-pass]] (an empty result is a
signal, not a pass).

**A fourteenth+fifteenth, 2026-10-29 (KI-340's class widened, one shared
backend) — three reds with ONE mechanism, and the third was the same shape
as KI-280 for the fourth time.** Deck-suggest, the rank route, and the
address test all reddened while `curl http://127.0.0.1:8901/health` said
ok: the machine has ONE embedding backend, and every parallel test thread
queued behind it past `EMBED_TIMEOUT` (10 s × 2 attempts). The deck tool
got `None` and printed "this machine has no embedding backend" — i.e. a
BUSY backend read as an ABSENT one; the rank handler outran its client's
30 s budget and the reply was EMPTY, which is why both rank panics printed
blank messages (`assert!(x, "{reply}")` on an empty reply). The fix is a
suite-wide gate, not more budget: a `cfg(test)` **tokio** mutex in
memory.rs (`EMBED_GATE`, `embed_gate().await` for async tests,
`embed_gate_blocking()` for plain `#[test]`s) held by every test that
embeds live — a std `MutexGuard` is refused by clippy
`await_holding_lock` when held across awaits, which is exactly why the
tokio mutex (its own suggested remedy) is the right one. One static spans
the class because cargo runs test binaries sequentially. Budgets widened
30 s → 120 s stay as the only-a-hang deadline. The lesson: a shared
SUBPROCESS/backend with a per-call timeout is a load amplifier — serialize
the callers in test code and never let a saturated backend answer "absent".

**An eleventh and twelfth, 2026-10-25 (the LayerFS arm's own, caught
by the gate run in one afternoon):** (11) *a fixture deleted under a
process-cached handle.* The stub CLI was per-test with cleanup, but
the driver's once-per-process gate caches the first resolution's path
— test A's `remove_dir_all` deleted the script test B spawns, the
spawn failed, and the panic landed **while holding `AMBIENT_LOCK`**,
poisoning the shared mutex so every later env test's `.unwrap()`
exploded into `PoisonError` (one flake became three). Fixes: one stub
per process, never deleted; assertions moved OUTSIDE the lock scope so
a failing test cannot poison it; new acquisitions recover with
`unwrap_or_else(PoisonError::into_inner)`. (12) *an env behavior-knob
is a cross-test radio.* `FLEET_LAYERFS_PROJECT` set process-wide for
one test's window switched the arm ON inside a CONCURRENT test's
delegation — a child pwd'd into a foreign test's mount path, the exact
collision the feature exists to prevent, manufactured by the harness.
Fix: the knob reads a per-thread override first (libtest: one thread
per test, dies with it), environment behind it, production path
byte-identical — and the tests never set the knob env at all. The
doctrine generalizes: a config env var is harmless to share, a
behavior env var must be thread-local or every test in the process
runs the feature.
