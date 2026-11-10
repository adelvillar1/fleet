# Session Recap — 2026-11-10: the failsafe that restores

**Summary.** The owner ruled the main session and its delegation chain
the failsafe: in the presence of a failure their goal is restoring
fleet to working order, never refusing work. The failsafe-restore plan
(`docs/plans/2026-11-09-failsafe-restore.md`) is **completed** — four
phases, nine criteria, all ticked, the gate green end to end — and it
shipped as 0.6.22 (`0222481`).

## The four phases

- **A — the diagnostic loop (C0).** `src/health.rs`: tiered probes
  (layerfs's worker port when provisioned, lane presence, docker at
  the slow tier, the API's key), the repair map naming each service's
  verb. The gui server runs the loop; `/api/state` carries
  `services`; the face paints the card and a transition repaints it.
- **B — degradation over refusal (C1–C3).** The child's two refusal
  sites became degradations: a workspace that cannot open runs the
  child on the main session's model with the plain launch folder; a
  route that cannot be prepared rides the parent's route; a routing
  config that cannot load degrades to the compiled-in declared order,
  named on the decision row. Pinned end to end:
  `a_dead_substrate_degrades_the_child_instead_of_refusing_it` (the
  KI-493 incident's own shape) and
  `a_child_routes_through_prepare_child_and_a_broken_config_never_refuses_it`.
- **C — the repair attempt (C4–C6).** A run over a provisioned-but-
  down substrate carries the repair note on its first request — the
  service, the cause, the verb, and the ruling (never refuse;
  degrade and repair) — logged as `Record::SystemNote`. An exhausted
  walk names every skipped candidate's cause and repair, and
  no-connectivity with no local model is named as the one
  unrecoverable failure.
- **D — the recovery mode (C7–C8).** Every settings write stamps the
  prior file beside the home; `fleet recover` restores what is
  broken from its stamp and names the probes' fresh verdicts.
  routing.json's `local_tail` puts a configured local model behind
  the deepseek tail as the chain's final rung.

## What the live proofs caught

- The note fired in agent and eval tests through the **real machine's
  dead substrate** — the note's first act was its own incident, three
  tests over. Fixed twice over: `repair_note_for_run` names only what
  the run depends on (provisioned, not "not provisioned"), and
  `AgentOptions.health_note` gates the note to the surfaces that can
  repair — the CLI and the face's run paths — while children, eval
  runs, and tests leave it off.
- One truncated segment edit removed ~9,000 lines of
  `tests/integration.rs` mid-session; rebuilt from HEAD plus the
  mechanical edits, every suite re-run green. The lesson: a
  windowed string replacement must assert its own boundaries.
- `the delegated workspace` test needed the folder override
  (`set_folder_for_test`) and a canonicalized registration key — the
  macOS temp symlink would have made the fixture miss its own
  registration silently.

## The pins

2120 lib tests, 171 integration tests, 3 eval scenarios, 1086 harness
checks across 135 cases; the gate's four legs green at the final
commit (fmt, clippy `-D warnings`, release build, whole suite). The
ledger is well-formed; KI-493's code side closed in its row (the
provisioning pull stays the owner's).

## Files changed

`src/health.rs` (new), `src/gui/{bridge,server? no — mod}.rs`, the
face (`app.js`, `app.css`, `index.html`), `src/agent.rs` (the note
seam), `src/session.rs` (`Record::SystemNote`), `src/route.rs` (the
config degradation, the local tail, the exhausted causes),
`src/subagents.rs` (the two degradations, `degraded` on the row and
the result), `src/settings.rs` (the stamps, `recover`),
`src/cli.rs`/`src/main.rs` (`fleet recover`), `tests/face/harness.mjs`,
`tests/integration.rs`, the feature docs (health, subagents,
routing), the ledger, the changelog, the three version declarations.

## Open questions / next steps

- **0.6.22 is committed, not delivered**: the dmg is not rebuilt, the
  app not installed, the face not relaunched — the delivery is the
  next session's first act if the owner wants it live.
- The owner's actual LayerFS repair (`docker pull node@sha256:3d27…`
  + `fleet install`) is still pending their go-ahead; the system now
  names it everywhere it bites.
- `TECHNICAL-DOCUMENTATION.md` and `FUNCTIONAL-SPECIFICATIONS.md`
  still owe their failsafe sections — the feature docs
  (`docs/features/health.md`, `subagents.md`, `routing.md`) carry the
  substance meanwhile.
- The plan-gate could not run during the draft (its provider chain
  answered nothing); the completed plan's criteria are ticked with
  test evidence instead.
