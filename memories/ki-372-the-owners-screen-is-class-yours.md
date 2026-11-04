---
name: ki-372-the-owners-screen-is-class-yours
description: "project 2026-10-30 (KI-372, 4753d13) — the paired phone's bot inbox listed 13 scheduled corpus-refresh runs as "needs you" because inbox_document skipped the class predicate every other owner surface applies; fixed to exactly class == yours, live 50 rows → 8, pinned by the_inbox_keeps_the_owners_rows_and_only_those"
type: project
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

The owner's first successful paired look at the bot face (after KI-371
let it render at all) was not "it works" but *"there is a long list of
things that say needs my attention that I don't remember that being the
case."* He was right — none of them were his.

**The defect.** `inbox_document` (the bridge builder behind the bot's
`/api/inbox`) built its rows straight from `sessions_read`, without the
`class` predicate every other owner-facing surface applies:
`work_class` (bridge.rs `work_class`) maps `kind: scheduled` →
`background`, `subagent` → `delegated`, `probe` → `probe`, and only a
channel-less, kind-less log is `yours`. The rail's own N0 rule, the
welcome's `owner_recent`, and the probe stamp all take exactly
`class == "yours"`. The inbox took everything. The live read was 50
rows, 13 `failed` — every one a `20261030T01…Z-…` scheduled
corpus-refresh run with `model: null`, `goal: null`, and a mid-thought
`last` line, because a dozen face restarts that day had killed them
mid-run → `interrupted` → the inbox's documented D5 mapping renders
`failed` → its strip counts failures as needing the owner.

**Why it matters beyond the row count:** the owner was looking at the
wreckage of the machine's own cron jobs and being asked to answer for
them. Two prior defects (a per-launch relay secret, the seal boundary)
had hidden the inbox entirely, so nobody had seen its contents since
the bot face shipped.

**The fix** (`4753d13`): `inbox_document` filters to `class == "yours"`
before the rows × 16 KiB tail bound and names the predicate as the
rail's own; `class` is passed through on the rows it keeps. An owner
conversation the face killed mid-run stays `failed` — that is the
mapping working, not a bug. Live after the restart: 50 rows → 8, all
`yours`, one genuine failure ("Right pane today: rename, archive, new —
no end verb…"). Pinned by
`the_inbox_keeps_the_owners_rows_and_only_those` (one owner log
survives; a scheduled run and a probe do not). Gate fmt/clippy/build/
test 0, harness 393/43. KI-369 was taken by a scheduled corpus job at
00:55Z, so this is KI-372 ([[a-plans-ki-number-can-be-overtaken]]).

**How to apply:** every new surface that lists sessions inherits the
owner-scope rule by name, not by accident — grep for the class
predicate before shipping a list, and when a second surface shows
"needs you" figures nobody recognises, read the rows' `class` field
first. Related: [[the-owners-work-is-the-view]],
[[bot-pwa-is-assistant-centric-not-work-centric]],
[[ki-371-the-sealed-frame-boundary]],
[[the-relay-session-is-durable-ki-370]].
