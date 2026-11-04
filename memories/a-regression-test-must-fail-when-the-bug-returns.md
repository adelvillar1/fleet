---
name: a-regression-test-must-fail-when-the-bug-returns
description: "feedback 2026-10-18 — a regression test can pass with the bug present and with the bug absent; the first form of the relay-spin guard passed both ways (the fake relay's hold bounded the rate either way, and it passed vacuously when no polls happened at all). Reintroduce the bug and watch the test fail"
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

While implementing the remote-access relay I wrote a guard for a real bug
(a loop that leaked a poll task per iteration until tokio's blocking pool
saturated at 512 threads). The test asserted "at most N polls in 1.5s"
and passed. Then I deliberately reintroduced the bug — and it **still
passed**, twice over:

1. The fake relay held each poll for 300ms, so the rate was bounded by
   the relay's hold whether or not the loop was spinning. The test
   measured the fake, not the loop.
2. It also passed when *no polls happened at all* — I had asserted only
   the negative (the bad rate never occurred) and never the positive
   (the thing under test actually ran).

The rewrite asserted the invariant directly — how many polls are in
flight **at once**, which is one for a correct loop and hundreds for a
spinning one — plus that at least one poll reached the relay. Only then
did reverting the fix turn it red.

**Why:** a regression test that cannot fail is worse than no test,
because it buys confidence the code has not earned. Writing the test
feels like the verification step, which is exactly why the bug in the
test survives: the author's attention is on the code that was just
fixed.

**How to apply:** after any regression test, revert the fix and run it.
If it stays green, the test is measuring something else — usually a
bound the fixture imposes (a sleep, a queue depth, a pool size) rather
than the invariant. And assert both directions: the bad thing did not
happen **and** the good thing did. Related:
[[verify-crypto-against-published-vectors]] (the same lesson one layer
down: a check that agrees with a wrong implementation whenever some term
is zero), [[fleet-memories-tests-need-their-own-home]].
