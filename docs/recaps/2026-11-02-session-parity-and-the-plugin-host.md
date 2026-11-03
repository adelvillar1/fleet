# 2026-11-02 — session parity, and the plugin host

## Summary

Five plans were implemented whole and landed: the provider-session-headers
and session-parity-remainders plans (Fleet's session-id machinery, from
the opencode `MissingSessionID` check through the resume-stable opaque
ids, the injection rules, the request-id capture, and the Responses
adapter), the usage-in-calibration plan (the usage columns, the anomaly
pass, the trajectory shape), the face-plugin-host plan (the bundle
format, the two trust tiers, the management surface, the evaluation
suite, and the promotion ladder), and the plugins-ux rethink (the
owner's "atrocious" verdict answered by three approved mockups
translated 1:1). The session spanned the Rust 1.99.1 toolchain move,
the Tauri app's first rebuild, and the repo going public.

## Plans worked on

### `docs/plans/2026-11-02-provider-session-headers.md` — per-provider session headers

Status after this session: **completed** (all criteria ticked with evidence).

The roster's `headers` field end to end: `{session}`/`{env:NAME}`
placeholders, the two-gate refusals (the settings writer and the router's
build), the names-only masking, the providers card's headers line.

### `docs/plans/2026-11-02-session-parity-remainders.md` — the remainders

Status after this session: **completed**. The Zhang/Khattab harness
paper's five gaps closed: resume-stable opaque UUID session ids seeded
per conversation (idempotent for re-routes), token-char/control-char
rules at both gates, the request-id allowlist capture on refusals, the
writer's static-value trap warning; then the Responses protocol adapter
(dialect-only — `store: false`, no `previous_response_id`, the full
history every turn, per hard rule 5), `{trace}`, and the W0 probe.

### `docs/plans/2026-11-03-plugins-ux.md` — the plugins surface rethought

Status after this session: **completed** (all seven criteria ticked).

The owner's "atrocious" verdict answered by three approved mockups
(`docs/plans/mockups/2026-11-03-plugins-ux/`) translated 1:1: the
built-ins as three grouped status-first sections (the tool plugin, the
lanes, the MCP servers — each row a status dot beside the name and the
one-line human state, each press opening the thing it names), the
install behind a door (the refusal beside the field, the elevation's
grant inline before the press), how trust works as a disclosure (the
two tier cards and the ladder), and the rail badge counting a waiting
draft.

### `docs/plans/2026-11-02-usage-in-calibration-and-stream-anomalies.md` — usage, anomalies, shape

Status after this session: **completed**. Increment A: the providers'
cache/reasoning fields riding the route rows (`billed_tokens()` the one
reading), the usage columns and spend card. Increment B: six
deterministic anomaly classes, at most two lane asks, zero on a dry run.
Increment C: the per-turn curve on a `ShapeRow` and `fleet calib
--shape` — the paper's rot question on Fleet's own traffic.

### `docs/plans/2026-11-02-face-plugin-host.md` — the face plugin host

Status after this session: **completed**. The bundle format and registry
(`plugins.json` through the writer's atomic seam), the `fleet plugins`
lifecycle, the rail's dynamic rows and the one-card management chains,
the two tiers' frames (sandboxed untrusted on the network-dead route,
plain privileged), the evaluation suite (the static pass, the declared
queries, `fleet plugins eval`), creator mode's drafts-never-self-applied,
and the promotion ladder gated on the suite with the registry recording
each elevation.

## Commits

| Hash | Message |
|------|---------|
| `3d246a1` | the roster's per-provider headers ride both request paths |
| `16c30e3` | the providers card names its headers and never a value |
| `3c1d32b` | the headers field in the contract docs, and the plan completed |
| `c554a32` | the conversation's session id rides the preparation, opaque and resume-stable |
| `a17e115` | the session lifecycle contract, and the parity plan completed |
| `c64d3fd` | the wire dialect rides the roster, and `{trace}` joins the placeholders |
| `509e0e3` | the Responses protocol as a wire dialect, spoken per model |
| `d816704` | the account card carries the provider's headers (remainders W4) |
| `5c0c5ab` | the wire dialect and the trace, and the remainders plan walked |
| `0f1399d` | Increment C joins the usage plan |
| `0253e89` | the usage rides the route rows (usage plan Increment A) |
| `7aa0544` | the stream's own anomaly pass (Increment B) |
| `43b37e5` | the run's trajectory shape rides the stream (Increment C) |
| `fbb47e6` | two folds with credit — the C8 candidates and the sse test's heartbeat |
| `0259bb4` | the usage columns, the anomaly pass, and the trajectory shape (docs) |
| `140c739` | the usage plan's nine criteria tick — completed |
| `dea8393` | the face plugin host's format, registry, and CLI (P0) |
| `a1b7f5c` | the plugin host's reads and routes (P1, host half) |
| `ca9a5cd` | the plugins surface mounts the chains (P1) |
| `eabe031` | Rust 1.99.1, and the plugin host's parked work |
| `07bb991` | the drafts awaiting review ride the C5 pin |
| `c8dde93` | the plugin host's ten criteria tick — completed |
| `6aa368f` | the plugins surface lists the built-in plugins (the owner's finding) |
| `2a8d0f6` | the state's plugin keys are flat, not nested (the live-face seam fix) |
| `7813311` | the plugins surface rethought — the answer first (plugins-ux) |
| `5f56b86` | the plugins-ux plan's seven criteria tick — completed |

## What was added

- **The session-id machinery**: the roster `headers` field with
  `{session}`/`{env:NAME}`/`{trace}` placeholders; the opaque,
  resume-stable, idempotent session-id derivation seeded per
  conversation; the injection-safe validation at both gates; the
  request-id allowlist capture on refusals; the writer's static-value
  trap warning; the providers card's names-only headers line.
- **The Responses adapter**: the request/response/streaming mappers in
  `api.rs`, dialect-only (`store: false`, no `previous_response_id`),
  per-model protocol routing at the walk, and the W0 probe harness
  (`scripts/responses-probe.sh`).
- **The calibration instrumentation**: the usage columns and spend card;
  `src/anomalies.rs` with six deterministic classes and the two-ask
  lane discipline; the `ShapeRow` and `fleet calib --shape`.
- **The plugin host**: `src/plugins/face.rs` (format, registry,
  lifecycle, promotion) and `src/plugins/eval.rs` (the suite); the
  `fleet plugins` verbs; the rail's dynamic rows and the plugins
  management surface; the sandboxed/privileged frames with the
  declared-query postMessage relay; the creator drafts ground; the
  conversation-statistics demo bundle (`tests/fixtures/plugin-demo/`).
- **The article verdicts**: the offloop super-agent mapping (in memory,
  earlier) and the Zhang/Khattab harness-paper mapping
  (`memories/harness-compositional-generalization-paper-vs-fleet.md`)
  — the latter's one adoptable idea became the usage plan's Increment C.

## What was fixed

- The two folds with credit: the C8 candidates in `capabilities.rs`
  (the measure refusal now carries `valid: array`) and the gui_sse
  test's `heartbeat` visibility — both the parallel session's in-flight
  breakage, folded so master's suite stayed green.
- The injection hole the first headers plan didn't cover: control
  characters in header names/values are refused at both gates.

## What was changed

- The `Usage` struct carries the providers' cache/reasoning fields
  (serde-defaulted, omitted when absent, the wire names mapped);
  `billed_tokens()` is the one place the reading is defined.
- The agent's per-turn curve survives on the `RunReport` (it was
  computed every turn and discarded).
- The rail's `GROUPS` declaration stays the one structure; the enabled
  plugins' rows join their declared groups dynamically.
- The Rust toolchain moved to 1.99.1 (the cleanup commit fixed its 41
  new lints; the gate re-verified green on the committed state).

## Files changed

**Session machinery:** `src/api.rs`, `src/route.rs`, `src/settings.rs`,
`src/capabilities.rs`, `src/gui/bridge.rs`, `src/gui/assets/app.js`.

**Calibration:** `src/calib.rs`, `src/agent.rs`, `src/anomalies.rs`,
`src/cli.rs`, `src/main.rs`, `src/explain.rs`, `src/lib.rs`.

**Plugin host:** `src/plugins/face.rs`, `src/plugins/eval.rs`,
`src/plugins/mod.rs`, `src/gui/server.rs`,
`src/gui/assets/index.html`, `tests/fixtures/plugin-demo/`,
`tests/face/harness.mjs`.

**Docs:** `docs/features/routing.md`, `docs/features/calibration.md`,
`docs/features/plugins.md`, `docs/features/gui.md`,
`FUNCTIONAL-SPECIFICATIONS.md`, `TECHNICAL-DOCUMENTATION.md`, the five
plan files, `docs/plans/mockups/2026-11-03-plugins-ux/` (three screens
+ the renders), and this recap.

## Doc updates applied

- `docs/features/routing.md` — the headers passage (three placeholders,
  the injection rules, the static-value trap, the refusal's id), the
  wire-dialect passage, the `{trace}` placeholder.
- `docs/features/calibration.md` — the usage columns, the anomaly pass,
  the trajectory shape.
- `docs/features/plugins.md` — the face-plugin host: the format, the
  two tiers, the ladder, the evaluation suite, creator mode.
- `docs/features/gui.md` — the `#/plugins` surface row.
- `FUNCTIONAL-SPECIFICATIONS.md` — the plugins verbs, the calibration
  flags and columns.
- `TECHNICAL-DOCUMENTATION.md` — the plugin-host bullet, the evidence
  spine gains `anomalies.rs`, the Keys bullet's dialect rule.

## Doc updates deferred (debt)

None — the plans' doc criteria were met in the landings. The named
pending halves are verification legs, not doc debt: the responses
probe's live run, the plugin demo's scratch-home creator run, the eval
suite's rendered browser-lane pass, and the usage plan's live provider
leg (the cache fields read back off a real call).

## CLAUDE.local.md changes

None this session.

## Open questions / next steps

Things left for future sessions:

- **The live legs**, each the named pending verification half of its
  plan: `scripts/responses-probe.sh` runs the moment a `/responses`
  provider and key are at hand; the plugin demo's scratch-home creator
  run and the eval suite's rendered browser-lane pass are the plugin
  host's; one real provider call reading the cache fields back is the
  usage plan's.
- **The beta shakeout** (`docs/plans/2026-10-30-beta-shakeout.md`) is
  the only active plan left; its B5 (pass three after the owner's own
  use) closes the beta gate.
- **The plugins-ux plan's own out-of-scope carry**: the second-brain
  panel, the lane cards, and the MCP servers card are the destinations
  the rows open — their own shapes are theirs, unchanged.
- **The stashes**: `stash@{0}`/`stash@{1}` hold the parallel session's
  in-flight churn from the investigation — their work, preserved.

## The session's second half (2026-11-03)

- **The plugins-ux rethink** (`docs/plans/2026-11-03-plugins-ux.md`,
  completed at `5f56b86`): the owner's "the plugin UI is atrocious,
  we need to rethink this" verdict, answered the settings-verdict way —
  the live capture grounded six gaps (the sixteen-row wall, the states
  as prose paths, no action where the state is, the machinery leading,
  the developer's install form, the tiers as prose), and the plan was
  **mockups-first by the owner's ruling**: three screens in the face's
  own idiom (`docs/plans/mockups/2026-11-03-plugins-ux/` — the grouped
  surface, the install door, the trust disclosure) approved before a
  line of implementation. The build translated the approved screens 1:1.
- **Two live-face findings the harness could not see**:
  - **The flat-shape seam** (`2a8d0f6`) — the bridge returned a nested
    `{built_ins, plugins, proposals}` wrapper the face read as one
    empty list; the harness's flat fixture matched the face, not the
    bridge, so the case passed at 679/0 while the live face showed
    nothing. The state document now carries the three keys flat.
  - **The built-ins themselves** (`6aa368f`) — the surface showed only
    installed bundles; the built-ins (the second-brain tool plugin, the
    lanes, the MCP servers) never rode the read. They now compose from
    the same documents the lane cards and the MCP card read.
- **The face's binary path moved**: the LaunchAgent now runs
  `~/.fleet/bin/fleet` (the installed copy), not the worktree's — a
  worktree rebuild alone no longer reloads the face; the fresh binary
  copies to `~/.fleet/bin/fleet` first. The served `/app.js` is the
  verification.
- **The Tauri app's first rebuild** (`cargo tauri build`, the owner's
  own command): `fleet-desktop` verified carrying the new face (the
  plugins surfaces embedded), the dmg at
  `fleet/target/release/bundle/dmg/` (9.41 MiB). The shell is the thin
  kind — its window navigates to the fleet binary's served face, so
  asset changes flow through the binary, and the shell rebuild matters
  for its own code.
- **The repo went public** (the parallel session's `5d4ee6e`, merged):
  the stale private claims and the version corrected.
- **The verification numbers at the close**: the gate 2,012/0; the face
  harness 703/0 across 67 cases (the plugins case rewritten to the new
  DOM — the sections, the dots, the presses navigating, the door, the
  disclosure, the badge).

## Notes

- **The parallel-session interleave was constant**: the shared worktree
  carried their commits beside mine all session — the KI-398 cleanup,
  the settings-ux completion, the toolchain move, and the two folds.
  The house's fold-with-credit pattern held: master's suite never
  stayed red.
- **The 1.99.1 alignment**: the cleanup commit moved the toolchain and
  fixed its 41 new lints; my re-run of the gate on the committed state
  (fmt 0, clippy 0, build 0, test 1,996/0, face 678/0) is the
  verification, not the commit message's claim.
- **The tower-log**: task `t-20261102-235109-d78a`, filed done with
  both plans' outcomes.
- **The cache sweep declined**: a build was in flight (the parallel
  session's); the daily-maintenance LaunchAgent covers it.
- **The memories**: the harness-paper comparison is saved; the
  offloop verdict predates this session in memory.
- **The mockups-first workflow is a pattern worth keeping**: the owner
  ruled "I want to see some mockups first" — three static HTML screens
  in the face's own palette (no build step, Chrome renders them
  directly) approved before the implementation. The build was a
  translation of the approved screens, and the design ruling happened
  where it is cheap.
- **The capture trap, recorded**: Chrome's `--screenshot` fires at the
  load event, before the state read's repaint — the pre-data paint
  reads as a bare page and is NOT the surface. `--dump-dom` with a
  timeout window quotes the loaded render; the harness's own VM case
  pins the handlers.
- **The El.find lesson, twice in one case**: the harness's `find`
  returns the matches array — `[0]` reads the element. Two of the
  plugins case's checks failed on the unindexed form before the third
  was written correctly.
