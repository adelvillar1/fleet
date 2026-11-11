---
name: session-close-hygiene-plan-drafted
description: "project 2026-11-06 — the session-close hygiene plan (docs/plans/2026-11-06-session-close-hygiene.md) is 8/8 COMPLETE and MERGED to origin/master (eb57d8f): Registry::shutdown + the run's close (the owner's \"if you spin it up, you shut it down cleanly\" principle in code), checkpoints::prune (the last 20 turns per session, 5 GiB store cap), `fleet hygiene` (the prune + the consolidation + the stale-process sweep), the GoalDone hook, the recap skill's 8d; live proofs quoted"
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

The owner asked (2026-11-06) for "computer memory and disk hygiene, memory
disposal and disk cleanup once a goal is completed and a recap is written" —
the follow-up to [[checkpoints-unbounded-72g-disk]].

**The owner's standing principle (2026-11-06, two steers).** "if chrome is
used during validation and confirmation of feature success, the agent that
spawned the chrome process needs to clean up after the work is done" and
"the principle is if you spin it up then you shut it down cleanly." See
[[spin-up-then-shut-down-cleanly]] — it supersedes the registry's old
left-alive-and-named run-end ruling.

**COMPLETE 8/8 and MERGED to origin/master at `eb57d8f` (commits `7e6b745`
+ `555392a` + `df693be`):**

- **`Registry::shutdown`** (`src/processes.rs`): stops every child the run
  spawned and still lives; the run's close calls it and the report names
  what stopped. The old left-alive ruling is rewritten to the principle.
- **`checkpoints::prune`** (`src/checkpoints.rs`): the retention — the
  last 20 turns per session, the store's newest 5 GiB, oldest first.
- **`fleet hygiene`** (a CLI subcommand, sync dispatch): the prune + one
  memory consolidation pass (deterministic, no model call) + the
  stale-process sweep. The plan's `scripts/session-hygiene.sh` became the
  in-binary command — hard rule 12.
- **The GoalDone hook** (`bridge.rs` event loop): the retention runs off
  the async context at every goal completion.
- **The recap skill's 8d** runs `fleet hygiene` at the session's close.
- **The stale-process sweep** (C5): stops only what fleet can prove it
  spawned — a command from the home's venvs or the browser lane's Chrome
  on its fixed CDP port (9333), with the parent gone. Live-proven: a
  spawned orphan was stopped by name.
- **The consolidation** (C3): the corpus's own deterministic cycle, no
  model call. Live-proven: the scratch home's honest empty.
- **C1's live proof**: the GoalDone hook's line — `fleet: hygiene: 5
  checkpoint turn(s) pruned, 1303 bytes freed` — quoted from the face's
  stderr after a real run completed a real goal, the store settling at 21.

**The sweep's parsing bug (caught by the live orphan, not by any test):**
`splitn(3, char::is_whitespace)` on ps's space-padded columns hands the
ppid slot an empty field — every line silently continued. The fix is
`split_once` twice with `trim_start()`. See
[[splitn-whitespace-runs-empty-field]].

**How to apply.** The pass is the prevention for
[[checkpoints-unbounded-72g-disk]] — the retention is live on the
reloaded face. The principle's family is
[[cleanup-pass-trash-not-rm-and-locks-are-runtime-state]] and
[[self-containment-is-a-standing-contract]]. Related:
[[delegation-during-goal-execution-deep-dive]] and
[[the-approval-wait-blocks-every-run-control]].
