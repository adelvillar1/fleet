---
name: fleet-repo-garbage-taxonomy-2026-10-26
description: "2026-10-26 the owner asked \"why do we have so much garbage in the repo?\" — the measured taxonomy (target/ 35 GB, tmp/ 20 MB with no retirement rule, memory churn that is bookkeeping) and the for-good fix landed the same day (4612e8d): daily-maintenance.sh on a LaunchAgent, generated/ corrected to the refresh driver and committed, 21 GiB freed"
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

Owner question 2026-10-26: "Why do we have so much garbage in the repo?"
The measured answer, classified by source — and every class then fixed for
good the same day (`4612e8d`) after the owner said "shouldn't we fix that
for good as well?":

- **`target/` — 35 GB.** Cargo's build cache, gitignored, grows
  monotonically; the sweep exists ([[fleet-recap-ends-with-cache-sweep]],
  15 GiB backstop) but only fired at a recap, and none had ended.
  **Fixed:** swept now (34.4 → 13.4 GiB, 21 GiB freed) and automated daily.
- **`tmp/` — 20 MB, gitignored, and the real junk drawer.** One-off
  face-audit scripts, `shots-*/` dirs, six generations of blend1/MCP
  refresh scripts, old logs — scratch lands there and nothing retired it.
  **Fixed:** 14-day retirement as a leg of the daily script.
- **The `git status` noise — the memory substrate working, uncommitted.**
  All 18 modified `memories/` files were pure bookkeeping (verified per
  file: zero content lines); 9 untracked files were new facts and
  `.archive/` moves; `src/delegate/tests.rs` was another session's WIP,
  deliberately untouched. See [[fleet-memory-corpus-dirties-the-tree]].
  **Fixed:** the corpus self-commits daily under `chore(memories)`,
  `memories/` paths only — first run landed 25 files (`4f31786`).
- **`generated/` — the near-deletion that taught the rule.** I classified
  it as "the one true orphan" and was ready to delete it; the reference
  check found the day-old memory `blend1-refresh-spawn-driver-recipe`
  calling it "the checked-in working driver" for the corpus refresh jobs
  — deleting it would have broken the documented recipe. **Fixed by
  committing it in place** (`4612e8d`), making the fact's name true, and
  by the daily script's fourth leg: untracked top-level entries are
  NAMED in `~/Library/Logs/fleet-maintenance.log`, never deleted.

**The for-good shape (`4612e8d`):** `scripts/daily-maintenance.sh`
(LaunchAgent `local.fleet-maintenance`, daily 09:20, the
agent-config-backup plist's pattern) runs sweep → tmp/ retirement →
corpus self-commit → orphan naming, logging to
`~/Library/Logs/fleet-maintenance.log`, never failing the machine. After
the commits, the only dirty file in the tree was the other session's
`src/delegate/tests.rs`.

**Why:** the owner reads the working tree as a signal of the work's
state; a junk drawer with no retirement rule reads as neglect even when
the tracked tree is spotless. And the near-deletion is the standing
proof of the older rule — look at the target before deleting, and a
memory fact's words about a path ARE the reference check.

**How to apply:** classify by source before deleting anything (build
cache, scratch, living-substrate churn, another session's WIP each have
a different right answer); propose a retirement rule for any scratch
location that only grows; treat an "orphan" as a question for the owner
until its references — including the corpus's own facts — are checked.
