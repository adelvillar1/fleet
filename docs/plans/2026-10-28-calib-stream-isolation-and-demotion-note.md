---
status: completed
created: 2026-10-28
updated: 2026-10-28
slug: calib-stream-isolation-and-demotion-note
---

# Plan: keep test traffic out of the real calibration stream; make the demotion note name its window

## Context

The owner asked why the router shows so many failures. The stream told the
truth once read whole (KI-341, KI-342, both found 2026-10-28):

- **KI-341** — `calib::resolve_root` (src/calib.rs) falls through to
  dev-decisions' convention (the `.local/share/dev-decisions/logs` tree
  under the user's home directory) for *any* home that carries no
  `calib.log_root` override, because the turn loop hands it the run's
  home plus the real user home from the environment. A test driving a
  routed run from a scratch home therefore appends its rows to the
  owner's real stream: 237 of the 347 route rows on 10/26–10/28 are one
  hash-identical 2-char task (`go`, `4cd0e21a9a0795a1`), judged fresh
  each time against the live sys1 lane and served by stepfun off the
  compiled-in table — real spend, from inside test binaries.
- **KI-342** — the demotion note (src/route.rs:2813) formats
  "N consecutive failures **today**" while `calib::demotions` reads a
  3-day window (`DEMOTION_WINDOW_DAYS`, KI-050), so two-day-old mimo
  decode failures presented as fresh and made the board read as
  ongoing mimo breakage. The owner's own week said otherwise, and the
  real rows agree: zero failed requests across the three days — every
  failure was an *attempt* the failover walk recovered.

Ground truth verified by code read this session:

- `resolve_root` legs: config override → devdec fall-through →
  `home/logs`. Leg 2 is the leak.
- Callers: the agent turn loop (home + the user home environment
  variable), boards.rs (real fleet home — must keep bridging to
  devdec), cron.rs's eval-minute runner (an eval home — isolation is
  *correct* there), doctor.rs (passes no user home, already on leg 3),
  the TUI's explain modal and panels (the real fleet home — keep
  bridging).
- Only one test pins `resolve_root` (the `/scratch` one with no user
  home); the identity gate preserves it. No test asserts the demotion
  note's wording.

## Approach

1. **Identity-gate the fall-through** (KI-341): leg 2 fires only when
   `home` *is* the user's fleet home — canonical-compare `home` against
   the user home's `.fleet`; any other home resolves to its own
   `logs`. The real run keeps writing beside devdec's own rows; every
   scratch home keeps its rows beside itself.
2. **Name the window** (KI-342): the note reads
   "N consecutive failures in the last D days", with both numbers from
   the calib constants.
3. **Prove both**: a regression test pins a foreign home resolving
   inside itself and the real fleet home still reaching devdec; the
   wording's test names the window; the full local gate runs green.
4. **Log honestly**: both KI rows close as fixed with the evidence;
   the 237 polluted rows age out of the 3-day window (they carry no
   failed attempts, so demotion counts stay clean); purge stays the
   owner's call and is out of scope here.

## Acceptance criteria

- [x] C1: A route row from a run whose home is not the user's fleet
      home resolves to that home's own `logs` tree — never to the
      user's dev-decisions tree — while the real fleet home still
      resolves to the devdec tree. Proven by the new regression test
      plus the existing `/scratch` test staying green.
- [x] C2: The demotion note names the window ("in the last 3 days",
      both numbers from `DEMOTION_THRESHOLD` / `DEMOTION_WINDOW_DAYS`),
      with the test asserting the new wording.
- [x] C3: `cargo build --all-targets` and `cargo test` exit 0 with the
      toolchain PATH prefix, exit codes read on cargo itself; KI-341
      and KI-342 rows updated to closed — fixed with this evidence.

## Files to be touched

- `src/calib.rs` — the identity gate + the regression test
- `src/route.rs` — the wording + its test
- `docs/known-issues.md` — the two rows' closure

## Out of scope

- Purging the 237 polluted rows (owner's decision; default is age-out
  by Oct 31 through the 3-day window).
- Any board/UI change: the demotion guidance reads the same
  `calib::demotions` and inherits the honest note unchanged.

## Verification

Per criterion: C1 by the new `resolve_root` tests (foreign home → own
logs; real fleet home → devdec) run inside `cargo test`; C2 by the
updated wording assertion; C3 by the gate's exit codes, read directly.

## The declaration

```graph
{
  "nodes": [
    {
      "approach": "Identity-gate the devdec fall-through in calib::resolve_root: canonical-compare home against the user home's .fleet; a foreign (scratch-test) home resolves to its own logs; update the resolution-order doc comment and add the regression test; the existing /scratch test stays green",
      "cost": 2,
      "id": "stream-isolation",
      "paths": [
        "src/calib.rs"
      ],
      "serves": "C1"
    },
    {
      "approach": "Format the demotion note as 'N consecutive failures in the last D days' with both numbers from the calib constants; pin the wording in a test",
      "cost": 1,
      "id": "demotion-wording",
      "paths": [
        "src/route.rs"
      ],
      "serves": "C2"
    },
    {
      "approach": "Run cargo build --all-targets then cargo test with the toolchain PATH prefix, exit codes read on cargo itself; close KI-341 and KI-342 with the evidence; set the plan's status in the same turn its work lands",
      "cost": 2,
      "id": "verify-and-log",
      "paths": [
        "docs/known-issues.md"
      ],
      "prereqs": [
        "stream-isolation",
        "demotion-wording"
      ],
      "serves": "C3"
    }
  ]
}
```

## Outcome (2026-10-28)

C1 and C2 landed in one pass: the identity gate (`is_the_user_fleet_home`)
and the window-naming note, each proven through the router itself per the
owner's steer — `the_devdec_fall_through_is_the_user_fleet_homes_alone`
and `the_walks_note_names_the_window_the_demotion_reads` (the latter
plants demotion rows in the scratch home's own stream, which only
resolves because C1's gate landed, then asserts the decision's reason
verbatim). Gate: build exit 0; suite exit 0 on the immediate re-run
(1594 passed) after KI-343's unrelated relay flake reddened once and
passed alone. KI-341 and KI-342 closed with the evidence; KI-343 and
KI-344 logged from the same session's findings. No commit — the working
tree awaits the owner's.
