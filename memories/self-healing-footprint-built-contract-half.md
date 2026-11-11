---
name: self-healing-footprint-built-contract-half
description: "project 2026-11-09/10 — \"make fleet self healing and smart about the environment, approved and build it\": the DETERMINISTIC half is BUILT and live in fleet footprint (process tree vs ceilings, the environment line with inferred appetite + reserve, admit/settle pure functions, 6 tests); the env series is recorded by footprint.sh --record; admission/settlement wiring around work, the sys1 arm and the calibration rows are NOT built (C2-C6, C8, C9); plan moved to in progress at bcc59cd"
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

The owner's approval (2026-11-09): *"let's make fleet self healing and smart
about the environment where it runs, approved and build it."* Built as plan
`docs/plans/2026-11-09-the-resource-contract.md`, now `status: in progress`
(`bcc59cd`).

**What is BUILT and live** (`fleet footprint`, `src/footprint.rs`, master
`bcc59cd`, doctored into `fleet doctor`'s `caches` line from an earlier commit):

```
memory       ok    13 MiB of 2048 MiB (11 of 13 in pid 8264)
processes    ok    2 of 80
environment  roomy — reserve 18 GiB of 460 GiB (inferred; 1 record(s), standing 25% free)
disk         the budget table is scripts/footprint.sh
```

- `Env::standing_free()` = the MEDIAN of the recorded free-fraction series, not
  the latest reading (one good day after a cleanup must not read as roomy).
- `appetite()` — `Tight` (<10% standing free) / `Balanced` / `Roomy` (≥25%);
  no history = Balanced. A tight machine gets the LARGER reserve share
  (12% vs 8% vs 4%) — habitually full tightens fleet, never relaxes it.
- `reserve_kib(total, env, declared)` returns the reserve AND its source
  (`declared` / `inferred`); a fraction of the volume, never an absolute.
- `admit(available, reserve, estimate)` → `{ok, headroom, needed}`;
  `settle(before, after, budget)` → `{delta, runaway}`. Pure, six tests.
- `scripts/footprint.sh --record` appends `~/.fleet/footprint-history.jsonl`
  (volume_kib, free_kib per record, capped at 90 lines) and keeps
  `~/.fleet/footprint.json` for the doctor's `caches` line.

**What is NOT built** (the healing half): wiring `admit`/`settle` around the
release path and the run loop, turn-boundary measurement during a run, the
single sys1 arm for an abnormal reading, `op=footprint` calibration rows, the
estimate learned from settlements, the outcome-grading of the judge. Until
they land `fleet footprint` tells the truth about the environment but does
NOT act on it — say so plainly rather than claiming self-healing.

**Field caveat:** one record is not a history — the appetite inference needs
days of readings before it means anything, which is why the line prints
`N record(s)`.

**"We need to build the whole thing" — the honest close (2026-11-09).** The
session was at its context limit, so the whole was NOT built and must not be
claimed. What was added instead: the build order written into the plan's
`## The next slice, in build order` (`ea78df2`), slice by slice — C2 the
release path settles first (costs knowable, crash most expensive), then C3 the
run loop's turn-boundary measurement, C5 the `op=footprint` rows before C4 the
sys1 arm (the judge cannot be graded without rows), then C8/C9 the learning and
C6 the runaway proof. Each slice names its file, its seam and its test. Two
decisions were raised as the OWNER's, not the code's: the reserve fractions
(tight 12% / balanced 8% / roomy 4% are proposals) and whether a deterministic
breach hard-stops a run or pauses it for a yes. An idle-time (off-peak) task
was offered to build the remainder unattended from the plan as the brief.

**The handoff packet (2026-11-09, `3105523`).** Because the session ran out of
context, the close was `docs/plans/2026-11-09-the-resource-contract-handoff.md`
(12.5 KB) — read it FIRST in the next session. It carries: the owner's eight
rulings verbatim; the "do not rebuild" table (every `src/footprint.rs` function,
the `fleet footprint` output, each script and what it enforces, the four ledger
rows KI-502..505); whose numbers are whose (the reserve fractions/ceilings are
PROPOSALS until the owner confirms them); the seven unbuilt slices with file +
seam + test; the delivery flow with its two landmines (tag and assets in one
breath — a co-tenant commit between them burns the release; `fleet update`,
never `cp` over a running binary); and TEN traps that cost the session real time,
worth reading before any debugging: the service worker/Chrome cache serving the
OLD `?v=<hash>` bundle after a rebuild (a hash navigation reloads nothing — use
`Page.reload {ignoreCache:true}` in a fresh profile and read
`document.scripts[last].src`), `git stash` reverting the script under test (it
started a stray gate build — extract the function to `/tmp` instead), `cargo
test --lib` alone failing ~41 agent/delegate tests because the mock-server
fixture is built by `--all-targets`, a doctor check that needs a binary on PATH
(`df` vanished under the install tests' restricted PATH — read statvfs), BSD
`sed` lacking `\|` (use `-E '(a|b)'`), `rm -rf` being gated in the tool layer
(prefer `cargo clean`/`git worktree remove`), the face's real command line
(`fleet gui --no-open --remote --relay …`, so `pkill` on the wrong pattern
leaves the old binary serving), the face harness's flat DOM (select by id, prove
placement via `parentNode`), clippy's `is_multiple_of`, and a co-tenant session
committing and leaving untracked plans in the same repo.

Related: [[a-budget-per-store-beat-clean-up-after]],
[[the-resource-contract-admit-work-settle-curtail]],
[[the-failsafe-restore-fleet-to-working-order]].
