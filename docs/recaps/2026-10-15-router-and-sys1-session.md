# Session recap — the reboot recovery, the sys1 thesis test, and the router's settled architecture (2026-10-15)

## What happened

The session opened mid-recovery: slice 20's three modules
(delegate/graph/nomenclature) sat uncommitted with a red gate after the
owner's Mac rebooted. It closes with the sys1 judgment lane proven live,
the router's architecture settled by the owner's rulings, and slice 21
planned with its honest half landed.

## Landed and verified

**1. Slice 20's cores recovered — `047b3d7`.** The gate was 725 passed /
2 failed. Both failures root-caused: nomenclature's recency multiplier
was dead code (`file_days` computed days-since-epoch where the
establishment formula's contract is age — every real file landed in the
0.25 bucket), and the guard's own test asserted judgement beside a
bare `judge` is flagged — four edits apart, which the same file's
near-exact test calls "a different word". Fixed the formula to compute
age (the max folds flipped to min: the newest carrying file is the
youngest), rewrote the test's fixture to establish `judgment` (the
real-tree spelling the drift targets), and corrected two doc examples
that named the wrong pair. Gate: 843 passed / 2 ignored, release exit 0.

**2. The step-5-preview switch (config, no code).** The roster already
carried stepfun; `STEPFUN_API_KEY` was sourced from the Hermes config's
env into `<FLEET_HOME>/.env` (value never printed) and stepfun pinned in
`~/.fleet/config.json`. Verified live: `route: stepfun (pinned)`, a real
model answer, and the calib row `route stepfun 1 attempts`.

**3. The sys1 thesis experiment — and the defect it found — `ee405c5`.**
`fleet judge` refused with "not the structured content it promises"; a
direct stdio-MCP probe replicating fleet's own spawn showed the lane
answering every call with `can only join an iterable`. Root cause
lane-side (KI-191, previously logged from the consumer side): api.py's
two ClassifyResult returns join `provider` when the MCP surface's
documented absent-provider path sends `None`. Fixed in the vendored lane
AND the upstream `~/Projects/sys1` working tree; the consumer half
landed too (parse_call reads a nested `result.error` as the lane's own
refusal, pinned by tests). Verified live after: the lane answers
(verdict escalate, providers glide+drex, ~1.1s) and `fleet judge` landed
a real judgment — **wrong**, because sys1 judged the workload
`standard_code` (→ router-kit) while the pin served stepfun.

The lane fix also surfaced **KI-202**: route's fallback test had been
passing only because the lane crashed (its empty specs map resolved the
real vendored lane). Fixed hermetically by injecting an unspawnable
spec; the unit suite no longer resolves the ambient lane.

**4. Slice 21's honest half — `2812ca0`, plan rewritten.** The owner's
diagnosis — "the router was using deepseek even though it's the last
option" — traced to the surface, not the walk: `fleet run` printed the
CLI default model before routing, and the session log's start record
carried it, so a zai-served run read as deepseek. Landed: the decision
carries its model (one `decision_at` helper), `fleet run` prepares the
route before creating the session (first line and log name the routed
provider+model; a failed route leaves no dead session), chat's banner
makes no model claim, and the walk line reports the attempts with the
no-key skips naming their env vars. Verified live: `route: stepfun
(pinned) · model step-5-preview`, `walk: stepfun served`, start record
`"model":"step-5-preview"`.

## The settled architecture (the owner's rulings this session)

- Fleet is a **self-contained harness on the deepseek-harness
  philosophy: everything is a plugin** — the router's parts are plugin
  config inside fleet, FLEET_HOME-loaded with compiled-in defaults.
- The **routing pattern is internal**: the zcode-router-kit's
  per-workload preference table (verified against the live kit at
  `~/.zcode/router`) is ported natively; the kit is not consulted at
  run time.
- **blend1 is the grounding provider** (Liquid's hosted lfm 24B over
  the specialized legs) and the router targets it for the `composed`
  workload, through the vendored lane as a lane-kind candidate.
- **deepseek becomes structurally unable to serve unpinned**: declared
  payg, targeted by no workload (the kit's `allowPayg: false` ported as
  policy).
- **The pin is a provider, and its purpose is quota**: the owner points
  the harness at a vendor carrying a large unused allocation; the router
  then picks that provider's model per the judged workload.

## What remains (contracted in docs/plans/2026-10-15-router-enforcement.md)

Slice 21's port: the plugin config (`providers.json` + `routing.json`,
validated, typed errors), the walk becoming the workload's own
candidate list at (provider, model) granularity, blend1 as the
lane-kind candidate, the gates (`minConfidence` 0.6 →
`defaultWorkload`), the provider-level pin, and the real-run
verifications. Then slice 20's wiring (D1/D2-row-writer/D3/D4/D5/D6/D7
hook + CLI surfaces). `master` is 11 commits ahead of origin, unpushed;
the gate is local-only (CI disarmed by budget, rustfmt/clippy absent on
this machine).

## The enforcement round (same session, later)

The port landed and the tree went green end to end: 896 passed / 2
ignored across ten suites, release exit 0, twice consecutively. The
round's shape was the fixture fallout of the ported table plus two real
findings:

- **The owner's ruling re-made the fixtures**: after the port, the eval
  corpus, the judge's grade fixtures, and the routing integration tests
  all failed because they named the old table's targets. The first pass
  re-based them by hand to stepfun — and the owner corrected it: the
  suite has to be DYNAMIC, because the preference table is
  configuration and will move (a vendor's quota, a new provider, a
  pin). The fixtures now DERIVE from the same source the grader uses
  (`workload_candidate`, through a shared `hard_target()` helper), so a
  fixture can never disagree with the table it is graded by; the
  mechanics tests (failover, demotion, the judged seam) DECLARE their
  own preference table — a `routing.json` in the test home or a
  `Vec<Preference>` passed to `select` — so they prove the mechanics
  whatever the default prefers. The ruling is recorded in the
  fleet-repo-conventions skill.
- **KI-203, a real defect the port carried**: `decision_from` computed
  the decision's `start_index` as the candidate's ROSTER position while
  the walk iterates it as an ORDER index, so any multi-entry preference
  whose head was not roster position 0 would start its walk mid-list
  and skip the candidate sys1 selected. The compiled-in table hid it
  (every multi-entry list's head sits at roster position 3 and every
  single-entry walk clamps to 0); the demotion test's declared table
  exposed it. Fixed to the order position — the value `prepare`
  already recomputed — and the unused parameter dropped.
- **KI-204, an honest flake**: one cron test failed once with
  `Exists("every-minute")` in a freshly created test home; every theory
  checked was disproven (unique homes, no leftover stores, 3/3 in
  isolation, two green full runs since). Logged as noted, watching.
- The two nomenclature real-tree proofs re-based to the grown tree:
  the compound-preference proof now answers `judge_route` (the agent
  loop writes the compound — the rule working on the real tree, the
  retired expectation recorded in the comment), and the honest-miss
  proof moved to a verified-absent nonsense pair. One failure was the
  guard catching this session's own recap: a backticked judgement in
  the recap had established the drift term through the doc channel.
- The slice-20 delegation halves landed through the two children:
  `a1aeaaf` (D4/D5/D7 — the bounded fan-out, steer/stop, the write-time
  guard) and `93c32e6` (D1/D6/D2-writer/D9/D10 — the structured brief,
  the n+2 depth model, the delegation row, the capability declaration,
  the delta report). The child is not stuck when its transcript shows
  commits landing; its gate run is its own verification, and a red
  build inside a sibling's uncommitted files is the shared-tree
  transient, not a defect.

Remaining in slice 21: the blend1 lane-serving path (AC4's
client-boundary half and AC9's composed leg). Then the delegation
plan's remaining wiring and CLI surfaces.
