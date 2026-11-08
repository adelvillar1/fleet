# Session Recap — 2026-11-07: the self-improvement program

**Summary.** The eight-area program from the DoubleAI deep dive
landed whole in a worktree (`../fleet-self-improvement`, branch
`self-improvement`, collision-free with the parallel sessions): the
demand map, the clustering beat, the analysis, the gaps, the run and
session scopes, the capability library, the self-probe re-run, and the
twin/sham race — ten criteria, ten commits, the gate green.

## Plans worked on

### `docs/plans/2026-11-07-self-improvement.md` — the self-improvement program

Status after this session: **completed** (all ten criteria ticked).

| Criterion | Status | Evidence |
|---|---|---|
| C0 — the demand map | ✅ | `src/demand.rs`, `fleet demand`; live: the subagent-delegation scenario reads 36% bad and worsening |
| C1 — clustering → shape proposals | ✅ | `fleet demand --cluster`; live: 7 clusters filed, every example naming the mock-server port |
| C2 — the self-probe re-run | ✅ | `fleet delegate --probe <key>`; the probe + the decision + the re-dispatch, tested |
| C3 — the twin/sham race | ✅ | `fleet improve race <name>`; live: the taxonomy patch ties the twin at 100% and is refused |
| C4 — analysis before policy | ✅ | `fleet improve analyze`; the measured threshold, the tool declaration names it first |
| C5 — the run/session scopes | ✅ | `judge --run`, `delegate --session`; the timing instrument |
| C6 — the eval gaps | ✅ | `fleet eval --gaps`, `--promote`; live: the delegation outcomes and validate verdicts uncovered |
| C7 — the capability library | ✅ | `fleet improve --capabilities`; the drafter's feedstock names them |
| C8 — the gate | ✅ | fmt 0, clippy 0 (all targets), build 0, test 0, harness 984/116 |
| C9 — the docs | ✅ | self-maintenance.md, calibration.md, the tech doc's module reference |

## Commits (the worktree, branch `self-improvement`)

| Hash | Message |
|------|---------|
| `c11ef8d` | feat(demand): the demand map |
| `4e35f67` | feat(demand): the clustering beat |
| `ca86897` | feat(improve): analysis before policy |
| `0a17f5d`+`bba79b8` | feat(eval): the gaps |
| `76ccaf5` | feat(beats): the run and session scopes |
| `40286e0` | feat(improve): the capability library |
| `377155a` | feat(delegate): the self-probe and the re-run |
| `0cfc10d` | feat(improve): the twin and the sham |
| `eb59714` | fix(clippy): the bin target's warnings |
| `46a14f6` | docs(self-improvement): the program's surfaces |

## What was added

- **`src/demand.rs`** — the phantom gradient's fleet instrument: the
  per-class demand over the calibration stream (rate, row count,
  trend), the clustering beat with the shape-proposal patch, the
  filing threshold.
- **The analysis** (`fleet improve analyze`) — the confidence threshold
  that separates right from wrong verdicts, measured from the stream.
- **The race** (`fleet improve race`) — the corpus on the applied, the
  twin, and the sham arms; the placebo test.
- **The self-probe** (`fleet delegate --probe`) — the outcome judged
  against the brief through the lane; a confident wrong re-dispatches.
- **The gaps** (`fleet eval --gaps`, `--promote`) — the uncovered
  classes and the scenario seeds.
- **The scopes** (`judge --run`, `delegate --session`) — the timing
  instrument.
- **The capability library** (`fleet improve --capabilities`) — the
  lesson store as what the system learned.

## What was fixed

- The socket guards caught two fixture violations (eval.rs and
  improve.rs constructing judged rows) — both rewritten as hand-written
  JSON lines, the route.rs precedent.
- The bin target's clippy warnings from the new code.
- The `run` command's `--session` arm, which a mis-targeted edit had
  briefly removed.

## Doc updates applied

- `docs/features/self-maintenance.md` — the program's seven surfaces.
- `docs/features/calibration.md` — the demand row and the scopes.
- `TECHNICAL-DOCUMENTATION.md` — the `demand.rs` module entry.

## Open questions / next steps

- The browser lane's Chrome discovery is environmental: the gate's
  test leg needs `BROWSER_CHROME` (or a PATH Chrome) on this machine —
  the same failure hits the main tree, and the earlier gates ran with a
  discoverable Chrome. The lane's error names the remedy.
- The 7 shape proposals filed by the live clustering beat sit in
  `~/.fleet/proposals/` — the owner's apply is the naming gate.
- The self-probe's re-dispatch is wired; the live probe path is proven
  on five real rows (four October environmental failures and one fresh
  false-premise delegation — the lane's verdicts at 0.42–0.60, all
  correctly under the 0.80 bar: an environment failure or a false
  premise is not a reason to re-run the child). The re-dispatch's live
  firing awaits a confidently-wrong verdict — the bar is the design's
  conservatism, and manufacturing one honestly is the next session's.
  The probe's lane path is pinned by the scripted-lane test; the
  decision by the should_rerun units.

## Notes

- The gate: fmt 0, clippy 0 (all targets), build 0, test 0 (with
  `BROWSER_CHROME` set — the environmental input), harness 984/116.
- The live proofs: the demand map (the subagent-delegation scenario),
  the clustering beat (7 proposals), the analysis, the gaps, and the
  race (the twin refusal) all ran against the real stream.
- The worktree's branch is pushed for review; the merge to master is
  the owner's call.
