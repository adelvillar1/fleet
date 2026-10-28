# The rail, the archive ruling, and the model estate

**2026-10-27** · the face caught up with its own rulings: the sessions rail
shows the present (`KI-334`), sessions end only when the owner archives them
(the day's plan, implemented and verified), the working folder became a
default instead of a ritual (`KI-337`), and the model estate took two
rulings — clef measured and *not* promoted, the `frontend` workload class
routed by name. Ledger checker green: **338 rows, 17 open**. No commits —
the day's work rides the working tree across fleet, sys1, and sem1, the
owner's to commit.

## The sessions ruling: end only by archive

**Plan `2026-10-27-sessions-end-by-archive.md` — status: implemented →
completed this session.** The trigger was a triage the owner asked for
directly: the sessions rail listed nothing after 2026-10-23 and the owner's
bot-PWA night session was unlisted. What the triage found was two defects
wearing one symptom:

- **KI-334 (the cap kept the wrong end).** `sessions_read` capped the archive
  with `take(SEARCH_BOUND)` over an oldest-first list — the OLDEST 200 logs,
  everything after 2026-10-23 cut from the rail and from `?q=` alike. Nothing
  was cached; the morning's restart proved it. The cap now takes from the
  tail; pinned by `the_rail_keeps_the_newest_search_bound_logs` (201 logs).
- **The vocabulary lied.** The loop stamped `End { stop_reason: "completed" }`
  whenever the model's reply carried no tool calls — the dogfood run had
  stamped itself `completed` while its last message was a question to the
  owner. The owner's ruling: *a user-launched session never ends by the
  agent's word; the only end is the owner's archive.* The loop now stamps
  **`handed back`** for interactive stops (scheduled/delegated keep
  `completed`, where it is true); recall signs handbacks Incomplete, never
  Divergence; and the face carries the **archive verb** — `POST
  /api/session/archive`, the `sessions-archived.json` registry, a collapsed
  Archived group, chips on every row, the head button, unarchive, and a
  refusal for archiving a run in flight. The face was verified live end to
  end, including the owner's own archive-then-restore.

## Test hermeticity: two leaks closed at the root

- **KI-335 (the machine's sweep state leaked into tests).** The lifecycle
  sweep had disabled `list_dir` in the machine's `tools.disabled`, and
  scratch-home Agent tests inherited it via the fleet-home fallback — a
  declarations test went red with no code change (proven pre-existing by
  stash). Fixed structurally: **the disabled set reads the run's own home**
  (derived from the session log path — production-identical, tests hermetic
  by construction), plus `set_tools_disabled_for_test` for simulating lists.
  First attempt via `improve_home` was wrong (it gates the `improve` tool's
  declaration); the memory records the wrong turn.
- **KI-336 (the eval corpus needed the machine's LayerFS worker).** The
  `subagent-delegation` scenario's child opens its workspace through LayerFS;
  with the worker down (the Docker-reset outage) the corpus reddened. Fixed
  the same way: **`resolve_project` reads the substrate from the run's own
  home** (`read_machine_at`), so scratch-home delegations run plain and the
  corpus grades the delegation contract with the worker up or down — proven
  green while the worker was still down. The six integration tests pinning
  the old end vocabulary were updated to the handback words in the same pass.

## The folder ruling: a default, not a ritual

**KI-337 — closed at three layers.** The owner set the working folder
repeatedly and every run still refused: the folder landed on the pending key
`""`, which only the launch session reads — and `load_workdirs`' prune
**dropped the pending key from memory on every start** (the file kept it, the
process never had it). Sessions fleet itself creates never got an entry at
all. Now: the pending key survives the load prune (the goals sidecar's fix,
one seam over); `GuiState::effective_folder` resolves the chat's own choice,
else the fleet default, and **both the run path and the state documents read
it** (the face shows what the run will use); the deny floor holds where the
owner left no folder anywhere. The owner's session was fixed directly and the
next message got file access.

## The model estate: clef measured, frontend routed by name

- **clef (Cloudflare's 27B decision model, Ollama cloud) — measured, not
  promoted.** The owner's ruling sent the rationalization through the plan:
  the provider landed in sys1 (the Jev-family System One wire, 10 tests),
  and the shadow A/B replayed 25 real route states against the chain's
  cached verdicts. **Workload agreement 32%, verdict agreement 56%** —
  systematic: all 16 cached `standard_code` states classified `quick`, and
  the pass rate doubled. The gate (≥90%/≥85%) failed; clef stays a
  registered, live-verified **bench candidate** with a named deficiency
  (coding-agent workload tiering). Report: sys1
  `docs/ab/2026-10-27-clef-shadow.md`; plan: sys1
  `docs/plans/2026-10-27-clef-provider.md`. The estate correction that
  framed the whole inquiry: the house has **no local System One models** —
  every decision seat is hosted.
- **The `frontend` workload class.** The owner's ruling: frontend/UX/UI work
  routes to **deepseek-flash** (the API reserve — first choice for THIS
  workload only, the payg guard's own "unless a workload names it" clause)
  with GLM-5.3-Flash as backup and **ollama-cloud glm-5.3-flash** as the
  final fallback — three rungs across three hosts. The label lives in sys1's
  route task (the description is the signal — the clef A/B proved labels are
  scored, not learned), the candidates in both the compiled table and
  `routing.json`, `profile_rank` treats frontend like standard_code, and the
  vendored sys1 was synced before the label could be judged. The route cache
  was flushed at the flip; the gui restarted onto the build.
- **sem1: the semantic lane is back (KI-338).** Recall had degraded to
  lexical: sem1 shipped a `probe.py` submodule **and** a `probe` function,
  and the function's own lazy import bound the module over the function —
  the second probe call in any process died with "'module' object is not
  callable". Renamed `probing.py`, regression test added (the suite is 21),
  the blend1 service restarted — three health probes green, `/v1/search`
  returning scored results. The vendored copy synced.

## Files changed (the working tree, uncommitted)

- fleet src: `route.rs` (frontend preference + roster entry + the ruled
  reserve exception), `agent.rs` (profile rank, the end-word),
  `session.rs` (Session.kind, the handback stamp), `session_recall.rs`
  (the Incomplete sign), `settings.rs` (the disabled-set pin),
  `subagents.rs` + `layerfs_tools.rs` (the run-scoped substrate),
  `gui/bridge.rs` (the cap, effective_folder, the archive registry and
  verb, work_state), `gui/server.rs` (the archive endpoint),
  `gui/assets/*` (the Archived group, the chips, the head button),
  `boards.rs` (the tier list + the payg guard's ruled exception),
  `tests/integration.rs` (the handback vocabulary), `tests/face/harness.mjs`
  (the ARCH case, 263 checks).
- services: `services/sem1/` (the rename), `services/sys1/src/sys1/tasks.py`
  (the frontend label).
- docs: `known-issues.md` (KI-334…KI-338),
  `docs/plans/2026-10-27-sessions-end-by-archive.md`,
  `docs/recaps/` (this file).
- outside the repo: sys1 (the clef provider + the A/B script + the report +
  the plan doc), sem1 (the rename + the regression test, uncommitted),
  blend1 (restarted, no code change).

## Doc updates — applied (the owner's ruling: update all docs, refresh the README)

All three proposed updates **applied**, plus the wider pass:

- `docs/features/gui.md` — the work-list section: sessions end only by
  archive, the full state vocabulary (`handed back`, `archived` among
  them), the chips, the in-flight refusal.
- `docs/features/routing.md` — the `frontend` tier with its three-host
  walk (deepseek-flash → GLM → ollama-cloud glm).
- `FUNCTIONAL-SPECIFICATIONS.md` — the folder default in Run behavior;
  the archive semantics as a run-behavior bullet; the rail's state
  words; and the stop-reason vocabulary aligned (`handed back` /
  `handed back without a report` in the table, scheduled/delegated
  keeping `completed`).
- `docs/architecture/lanes.md` — the judge line names frontend.
- `docs/architecture/surfaces.md` — twelve destinations, the
  workspace-history addition named with its date.
- **README refreshed**: the lanes are now an at-a-glance table (sys1/
  sdm1/sem1/gen1/blend1 — what each does, what it returns) pointing at
  the case doc; **LayerFS moved into What ships today** (two bullets:
  collision-free delegation + the workspace-history instrument — it was
  built, it was wrong to list it as next); the sessions-end-by-archive
  and folder-default bullets; the frontend tier in the router bullet;
  the surface table gains workspace history (twelve).
- **Screenshots re-captured with Playwright** (system Chrome, headless,
  1440×900 @2x — the house viewport): all twelve surfaces replaced
  (`docs/images/surface-*.jpg`, fresh 2026-10-27), including the
  sessions surface showing the new world — `handed back` rows, archive
  chips, the Bot PWA session listed — and the conversation shot lived
  in the owner's own session. One session was archived temporarily for
  the shoot and restored; the estate is unchanged. `TECHNICAL-DOCUMENTATION.md`:
  **no update needed** — its routing summary names the sys1 judgment
  and the failover tail, which still hold; the archive endpoint is a
  face-level addition the gui feature doc covers.

## Open questions / next steps

- **Commits**: fleet, sys1, and sem1 all carry the day's work uncommitted —
  the owner's. The `services/sem1/VENDOR.json` and `services/sys1` pins
  update to the shas at re-vendor after those commits.
- **The blend1 sem1 probe fix is live but blend1 has no commit either** —
  the rename lives in the sem1 repo; blend1 needs nothing.
- **The escalation leg** stays unconfigured by design: it is a generative
  chat lane, and clef (non-autoregressive) cannot fill it — the correction
  is recorded in the clef plan.
- **`vision_qc` stays on liquid** — the clef A/B was text-only; an image
  smoke probe is the door to moving it.
- **The workspace-history screenshot is the honest empty state** — the
  LayerFS worker is still down (the Docker reset), so the instrument
  renders its Store-empty card and names the refused reads. After
  `fleet install` re-provisions and a delegation runs, re-shoot it with
  the tree populated: same capture script shape, `#/workspace-history`.
- **clef's re-entry path**: a criteria-encoding iteration on the workload
  head, or an escalate-class A/B against jev alone. The bench holds the
  choice, per the ruling.

## Notes

- Cache sweep at close: **debug 16.6 GiB → 13.8 GiB**, free space 89.4 →
  92.2 GiB (release untouched).
- No Hermes process registry on this machine — fleet replaced it; the
  session's processes are the three gui restarts and the blend1 restart,
  all intentional and running.
- The A/B's candor note, worth keeping: the cached verdicts are the
  chain's answers, so the 32% measures clef against production — and the
  systematic `standard_code`→`quick` downgrade is a criteria-encoding
  question, not noise.
- Memory written this session: the rail cap correction, the folder
  default ruling, the frontend workload checklist.
