---
name: a-second-face-must-share-the-firsts-parsers
description: "feedback 2026-10-28 — the remote face's /api/run called bridge.run (mode-less) and never parsed the body's mode, so every bot run ran in Ask mode asking on every call while the console's route parsed correctly; a second surface that parses its own request bodies drifts silently — share the parser"
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

The bot-PWA dogfood (2026-10-28) found it live: a non-destructive bash
call raised an approval seat under a run posted as guarded. The gate was
fine — the **remote face's `/api/run` route called `bridge.run(&session,
&task)`, the mode-less constructor**, and never read the body's `mode`.
Every bot run ran in `Mode::Ask`, asking on everything, while the
console's route (`run_policy_of` → `run_with_budget`) parsed the same
body correctly. C7 ("non-destructive work runs without asking") passed
its tests and was dead on the phone face.

**Why:** a second listener that serves its own route table re-implements
request parsing, and whatever the first face gained later — a mode word,
a budget field, a folder rule — silently never reaches it. The two faces
shared the bridge but not the PARSERS, and the parser is where the
contract lives.

**The fix:** `run_policy_of` / `turn_budget_of` / `folder_of` were made
`pub(crate)` and the remote route calls them, then
`bridge.run_with_budget` — one parser, two callers. The rule going
forward: when a face gains a body field, grep the OTHER face's route for
the same path; and when a second face is built, its routes call the
first face's parsers rather than copying them.

**How to apply:** a defect that only shows on one surface, with tests
green, is a surface-drift smell — diff the two routes' body handling
before suspecting the gate or the mode. Related:
[[bot-pwa-dogfooded-live-01a9ab3]], [[a-reducer-dropped-case-goes-silent]]
(the same drift shape on the client side),
[[a-read-fixed-is-not-a-view-fixed]].
