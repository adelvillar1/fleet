---
name: ki-379-warmup-hang-2026-10-30
description: "KI-379 (closed into KI-380) — the ~30-min warmup "hang" was an invisible approval seat behind a zombie page; the embed-flap diagnosis was a red herring; sample the pid first, and check /api/state's seat before any server theory"
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

CORRECTED 2026-10-30: the row's first diagnosis was wrong. The ~30-min
"hang" and the owner's second report ("nothing is happening the session
warmup is stuck again, major regression") were ONE incident — an
approval seat the owner could not see. See
[[ki-380-zombie-tab-2026-10-30]] for the full chain and the unstick.

What the first diagnosis got right: capture evidence BEFORE restarting
(the process state, the endpoint timings, the log tail — all captured),
and the runbook stands for REAL server hangs: `sample <pid> 30` before
restarting, the blocked frame names the layer.

What it got wrong: the corpus-embed-under-KI-296's-flap arithmetic
(114 facts × 10 s ≈ 19 min) fit and was still false — the embed server
answered 200 in 27 ms throughout. The truth was in `/api/state`'s
`approval` field all along: a standing seat for `tool_search`, its
`since` predating the report by hours. **Lesson: when the server is
healthy and the page is stuck, read `/api/state`'s seat BEFORE any
server-side theory** — an ask-mode run waits on the owner by design,
and the only question is why the owner can't see it.

Closed into KI-380 (2361023, pushed). The face was relaunched as asked
(pid 97613, all reads 200, serving 0.3.0-beta — confirmed by the
binary, not the HTML, which carries the git-hash build stamp, not the
version).
