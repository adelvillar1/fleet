---
name: spin-up-then-shut-down-cleanly
description: "owner principle 2026-11-06 — \"if you spin it up then you shut it down cleanly\": the agent that spawns a process owns its shutdown, the Chrome a validation run drives included; SUPERSEDES the old run-end ruling that a background child is \"left alive and named\"; landed in code (Registry::shutdown + the run's close at 7e6b745)"
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

The owner's principle (2026-11-06, two steers while the session-close
hygiene plan was being shaped): "if chrome is used during validation and
confirmation of feature success, the agent that spawned the chrome process
needs to clean up after the work is done" and then, generalizing it: **"the
principle is if you spin it up then you shut it down cleanly."**

**What it supersedes.** The `process` registry's documented run-end
behavior was the opposite: "a child still running when the run ends is left
alive and named in the report, because killing a server the owner started
is worse than leaving it" (`agent.rs:2772-2780`, `processes.rs:132`). The
principle replaces that ruling — the run's close now SHUTS DOWN what the
run spawned and the report names what stopped
([[session-close-hygiene-plan-drafted]], C0, `7e6b745`).

**Why to apply.** The owner's framing is ownership, not caution: whatever
the agent spun up — a Chrome driven for validation, a dev server, a
watcher — is the agent's to terminate when the work ends, and leaving it
running is the defect. The lanes already closed on the run's exit and the
browser lane already terminated its own Chrome; the gap the principle
closed was the `process` tool's children and the orphans a lane's death
leaves. The machine-wide sweep of what escaped is the hygiene plan's C5 —
LANDED and live-proven (a spawned orphan stopped by name).

**How to apply.** Any new surface that starts a process must end it: a run
closes its children, a goal's completion triggers the retention
([[checkpoints-unbounded-72g-disk]] is the disk-side twin), and a cleanup
that kills must name what it stopped so the owner reads the shutdown
instead of finding the processes later. Same family as
[[a-declaration-is-not-an-execution]] (a principle is not landed until the
code carries it) and [[self-containment-is-a-standing-contract]].
