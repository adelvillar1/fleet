---
name: fleet-recap-ends-with-cache-sweep
description: "the cache sweep (2026-10-21 3690ffe, KI-279's 159 GiB) — automated 2026-10-26 (4612e8d) inside scripts/daily-maintenance.sh on LaunchAgent local.fleet-maintenance daily 09:20, with tmp/ retirement, the memories/ churn self-commit, and untracked orphans named never deleted; release never touched"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The gate died on `No space left on device` on 2026-10-21: `target/debug`
held **159 GiB across 668k files** (`du` said 45 GiB — APFS clones hid the
rest) because cargo never deletes a stale artifact and six days of heavy
development left every generation of test binaries behind. The fix is a
standing step, not a one-off: `scripts/cache-sweep.sh` ends the recap step,
enforced in CLAUDE.md's session protocol, docs/harness-orientation.md step 4,
and the write-session-recap skill's new step 8c (so it applies to every
project).

A second standing prerequisite landed 2026-10-26 (owner ruling): the recap
document is not generated until `python3 scripts/ki_ledger.py
docs/known-issues.md` reports zero problems — in the skill's pre-flight and
as step 8's gate, and named in CLAUDE.md's session protocol. A row missing
its next-action cell, or one carrying an unescaped pipe, shifts its cells so
the checker reads the wrong column as the status; the 2026-10-26 pass found
KI-316 and KI-317 — genuinely open — reading as closed behind the damage.

The sweep's shape: files under `target/debug` older than 7 days
(`FLEET_SWEEP_DAYS`) are pruned first, so a week's incremental state survives
and the next build stays fast; then a **tiered** 15 GiB backstop
(`FLEET_SWEEP_THRESHOLD_GB`) — a heavy week fills the cache faster than a
week ages it out, so yesterday-and-older go next, and only when today alone
is still over the line is the dev profile cleaned whole. `target/release` is
never touched (`~/.local/bin/fleet` symlinks into it and the face's
LaunchAgent runs that binary); a running cargo/rustc makes it decline;
`--all` sweeps every `~/Projects/*/target`; the report is in free-space
terms. All three tiers verified against a scratch target
(`--target <dir>`) before the live run. Logged as KI-279, commit `3690ffe`.

**Why:** the disk being 95% full blocked the gate mid-slice; a recap-time
sweep caps the damage at ~15 GiB per project without slowing builds.

**How to apply:** quote the sweep's last line in each recap's notes; if a
project has no sweep script but has a `target/` or similar cache, propose
adding one rather than deleting ad hoc. The sweep only fires at a recap —
it is not scheduled; for a heavy stretch between recaps,
`scripts/cache-sweep.sh --dry-run` measures it. Related:
[[fleet-owner-dogfooding-phase]].

**Recurrence confirmed 2026-10-26:** `target/` measured **35 GB** — over
double the 15 GiB backstop — because the sweep runs only at a recap and no
recap had ended since the heavy stretch (the same session's LayerFS work
rebuilds the suite many times a day). The owner noticed and asked "why do we
have so much garbage in the repo?". The standing fix needs a trigger that
does not depend on a recap ending; until then, a mid-stretch `--dry-run`
plus a manual sweep is the honest move. See
[[fleet-repo-garbage-taxonomy-2026-10-26]] for the rest of what the owner
was looking at.

**Automated 2026-10-26 (`4612e8d`) — the sweep no longer depends on a
recap being written.** `scripts/daily-maintenance.sh` (LaunchAgent
`local.fleet-maintenance`, daily 09:20, the agent-config-backup plist's
pattern) runs four legs, logging to
`~/Library/Logs/fleet-maintenance.log`, never failing the machine: (1)
the cache sweep; (2) `tmp/` entries older than 14 days retired (the
scratch drawer finally got the same rule target/ had — it had none and
only grew); (3) the **memories/ churn self-commits** under
`chore(memories)` — recall counters, supersede fills, archive moves,
new facts, memories/ paths ONLY (the explicit-path rule), safe because
every corpus file is written whole and guarded, so any snapshot is a
complete fact; (4) untracked top-level entries are NAMED in the log and
never deleted. First run: the sweep freed 21 GiB (34.4 → 13.4 GiB), the
corpus commit landed 25 files, and the guard named the maintenance
script itself (uncommitted at that moment) — the live-tree proof.
**The correction that shaped leg 4:** I classified `generated/` as the
repo's one true orphan and nearly deleted it; the reference check found
the day-old memory `blend1-refresh-spawn-driver-recipe` calling it "the
checked-in working driver" for the corpus refresh jobs — deleting it
would have broken the documented recipe. It is committed in place now
(`4612e8d`), making the fact's name true. Rule reaffirmed at cost of
being wrong once: look at the target before deleting, and a memory
fact's words about a path are the reference check.
