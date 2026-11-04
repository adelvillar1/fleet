---
name: react-to-cancelled-foreground-calls
description: "owner correction 2026-10-19 — when a foreground call is cancelled, react to the cancellation explicitly (say so, re-launch the work in the background, report when it lands); a cancelled call you silently re-run or ignore leaves the owner with no signal whether the work will ever finish"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner, mid-session: "let me stop you right there, if the foreground call is cancelled
and you don't react to that action you are stuck with no signal that the call is nor not
going to finish" — and asked "why are you stuck?" The concrete case: the full cargo gate
(1121 tests plus integration suites, 2-3 minutes) ran in the foreground, got cancelled,
and my next visible move was another long foreground call — from the owner's side the
session just went silent with no statement of what happened or whether anything would
finish.

**Why:** a cancellation is an event that demands a reaction, not a private inconvenience.
From the owner's chair, an unacknowledged cancel is indistinguishable from a hang — and
"hanging with no signal" was the exact complaint they had about fleet's own run loop
(KI-248) minutes later.

**How to apply:** when a foreground call is cancelled: (1) say plainly that it was
cancelled and what it was doing; (2) re-launch the work with `run_in_background` so it
can finish without holding the turn; (3) report the result when the notification lands.
Better still, launch known-long work (the full gate, a real `fleet run`, a model call)
in the background from the start and say so. Related:
[[fleet-gate-exit-code-trap]], [[fleet-run-control-and-timeout-gaps]].
