---
name: relaunch-is-not-a-code-event
description: "The owner's ruling 2026-10-29 — once the work is tested and the binary built from the verified tree, relaunching the app is mechanical; a new test run is only owed when the tree changed since the verification"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 12
superseded-by:
valid-until:
---

The owner's ruling (2026-10-29, after a gui restart session): "if you
finish the work and complete the tests but forgot to relaunch the app,
relaunching it shouldn't require a new test run."

**Why:** verification attaches to the code state, not to the deploy
act. A restart serves the same bytes the gate already proved; re-running
the suite to relaunch spends minutes re-deriving an answer already
held. The gate-before-ship rule verifies the tree once; deployment is
not a code event.

**How to apply:** sequence work as build → test → relaunch, each once.
Relaunch freely once the tree is verified. Re-verify only when the tree
or binary changed since the verification — that is a new artifact, and
the old green no longer describes it. Never wrap a deploy step in the
full gate, and never bundle another slice's verification debt (e.g. the
uncommitted resume-record work) into your own deploy step — each job's
tests belong to that job's close-out. See [[verify-fix-actually-ran]]
and [[change-type-conditional-verification]].
