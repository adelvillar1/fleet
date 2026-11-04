---
name: a-verification-with-no-output-is-not-a-pass
description: "feedback 2026-10-18 — a verification step whose filtered output came back empty is a failure signal, not a pass; re-run the command with the right invocation and read the actual result line"
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

While verifying the session recap's numeric claims I ran the fourteen
new tests through a filter, grepped the output for `^test result`, and
got nothing back. The next tool call reported the *other* checks as
verified and moved on — the empty result was silently absorbed. It
surfaced only because the recap-verification summary listed "14 new
tests" with no number beside it, which prompted a re-run.

The root cause was mundane: `cargo test --lib -- <name1> <name2> …`
needs the `--` before more than one positional filter. Without it cargo
rejects the second positional and errors — which is what happened the
first time I made this same mistake earlier in the session, loudly. The
second time the invocation was wrong differently and the command
produced output my grep did not match, so there was no evidence either
way and no error either.

**Why:** a filter that matches nothing returns exit 0 and an empty
stream, which is indistinguishable from "I never looked." The repo's own
rule (KI-014, [[fleet-gate-exit-code-trap]]) is about the exit code
surviving a pipe; this is the content half of the same trap — the exit
code can be 0 while the grep swallowed every line of evidence. The
general form is that a verification is not done when the command
returns, it is done when you have read the result.

**How to apply:** when a verification's filtered output is empty, treat
it as a failed verification and re-run, never as a pass. Read the actual
`test result:` line and its counts rather than the presence of output.
And when a document claims "N tests pass," confirm you have seen N — an
unnumbered claim is the tell that the check was never read. Related:
[[a-regression-test-must-fail-when-the-bug-returns]] (the other
vacuous-pass shape), [[fleet-gate-exit-code-trap]] (the exit-code half).
