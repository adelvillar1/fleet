---
name: checkpoints-unbounded-72g-disk
description: "project 2026-11-06 — the drive hit 100% because ~/.fleet/checkpoints holds 72 G with NO pruning anywhere (every turn = a 64 MB snapshot); the retention (checkpoints::prune, KEEP_TURNS=20, STORE_CAP_BYTES=5 GiB, `fleet hygiene`, the GoalDone hook) is NOW MERGED TO MASTER at eb57d8f; the store refilled twice during the session (69 G each time) before the merge landed"
type: project
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

The owner reported (2026-11-06) that "temp garbage files are filling up my
drive again". The survey found the volume at **100% — 2.7 GiB free on
460 GiB**, and the standout was **`~/.fleet/checkpoints` at 72 G**.

**The cause: checkpoints were unbounded.** Every turn captures a snapshot of
the working tree (`agent.rs:4330`, `crate::checkpoints::capture`),
`src/checkpoints.rs` capped a snapshot's own size (`MAX_FILES = 2_000`,
`MAX_BYTES = 64 * 1024 * 1024`) but had **no total cap and no retention**.
The live session alone held **509 turn directories × ~68 MB = 34 G**; the
older sessions added ~38 G.

**The other large items** (read 2026-11-06): `~/.cache/uv` 23 G (reclaimable
with `uv cache prune`), `~/.cache/huggingface` 16 G (the lane models —
leave them), the repo's `target/debug` 16 G, and `~/.fleet/venvs` 3.4 G.

**The first cleanup RAN (the owner's "yes, do 1-3"):** checkpoints kept the
last 20 turns per session — **984 turn directories deleted, 72 G → 15 G**;
`uv cache prune` **23 G → 13 G**; `cargo clean --profile dev` **target/debug
16 G → 0**. Free space **2.7 GiB → 78 GiB**. Note: `cache-sweep.sh` printed
"the dev profile was cleaned whole" but its own `cargo` lookup failed (cargo
is not on PATH; the line lives in CLAUDE.local.md) — the clean only worked
run manually with the rustup PATH export.

**IT REFILLED TWICE THE SAME DAY** — the live burn-down run refilled the
store to **69 G** twice while the docs-branch work was in flight; the same
owner-authorized prune re-ran both times (710 dirs, then 821 dirs). Each
new worktree's own `target/` also costs ~8-15 G.

**THE RETENTION IS NOW ON MASTER (2026-11-06, merged at `eb57d8f` — see
[[docs-install-tui-branch-state]] and [[session-close-hygiene-plan-drafted]]):**
`checkpoints::prune` (each session keeps its last `KEEP_TURNS = 20`, the
store its newest `STORE_CAP_BYTES = 5 GiB`, oldest first by mtime), `fleet
hygiene` runs it and names what it freed, the **GoalDone hook** runs it at
every goal completion off the async context, and the recap skill's 8d runs
`fleet hygiene` at the session's close. Pinned by
`prune_keeps_each_sessions_last_turns_and_the_stores_cap`. The face is
reloaded with the merged binary — **the retention is live**.

**2026-11-07, the enforcement question:** the store hit **69 G a further
time while the merged retention rode the SERVED binary** (the face at 3080
was rebuilt from post-eb57d8f trees twice before the refill was found) —
merged is not enforcing. The suspect: the GoalDone hook fires only at goal
completion, and the burn-down's long runs are exactly that shape, so either
the hook is not firing or the 5 GiB store cap does not enforce on capture.
**Audit `checkpoints::prune`'s callers and the capture path before trusting
the retention again**; the manual prune (last-20-per-session) remains the
working fallback.

**How to apply.** The checkpoint store is a rewind substrate, so retention
keeps the recent turns and drops the old — a per-session last-N is the
shape. At 100% full the **Bash tool itself cannot start** (the harness's
exec-dir mkdir dies with ENOSPC, which reads as every tool being broken) —
a transient gap let one command through, so retry before assuming lockout,
and the prune command is small enough to fit that gap. Same family as
[[verify-the-binary-is-newer-than-the-source]] and
[[cleanup-pass-trash-not-rm-and-locks-are-runtime-state]]. Related:
[[why-the-goal-run-never-delegated]] — the live session whose checkpoints
are the 34 G.
