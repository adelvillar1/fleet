---
name: face-api-session-in-query-string
description: "feedback 2026-11-06 — the face's /api/run takes the session from the QUERY STRING (`?session=<id>`), not the JSON body; a POST with the session in the body silently mints a fresh session and the run lands on the wrong log; also the release binary must be rebuilt after the wiring lands (the first live-proof face ran the pre-wiring build and the mechanism silently didn't fire); and the face's serial command loop cannot start a new run while a previous run is in flight — deleting the in-flight run's log does n…"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Three traps caught during the route-judges-the-work plan's C7 live proof
(2026-11-06), each costing a full live-proof attempt:

**1. The session rides the query string, not the body.**
`asked_session(bridge, request)` reads `decode_query(&request.query, "session")`
(`src/gui/server.rs:202-210`) — the JSON body's `session` field is read by
OTHER routes (goal, folder) but NOT by /api/run. A POST to
`/api/run` with `{"session": "<id>", "task": "..."}` in the body silently
calls `bridge.launch()` (the session param reads empty), which **mints a
fresh session** — the run proceeds on the new session's log while the
named session's log sits untouched. The correct form:
`POST /api/run?session=<id>` with `{"task": "..."}` in the body.

**2. The release binary must be rebuilt after the wiring lands.** The
first live-proof face ran a binary whose mtime predated the route
wiring — the strings check (`strings target/release/fleet | grep -c
"plan execution verdict"`) returned 0, proving the binary was stale.
The face ran, the run proceeded, and the mechanism silently didn't
fire. The rebuild is 3m40s; the strings check is instant.

**3. The face's serial command loop blocks on an in-flight run.** The
face processes commands serially — a second /api/run is accepted and
queued but does not start until the first run ends. Deleting the
in-flight run's log file does not unblock the loop (the agent holds the
file handle); the only way out is the control file (`{"stop":true}`),
killing the face, or waiting. A live-proof setup that wipes and
re-drives must kill the face FIRST, then wipe, then restart — not wipe
under a live run.

**How to apply.** When driving the face's API from a script, read the
route's own handler to find WHERE it reads each parameter (the query
string and the body carry different fields per route). When a live
proof's mechanism silently doesn't fire, check the binary's strings
for the mechanism's own constant before debugging the code. When
re-driving a proof, kill the face before wiping any state. Same
family as [[verify-the-binary-is-newer-than-the-source]] and
[[a-fixture-can-match-the-face-and-miss-the-bridge]].
