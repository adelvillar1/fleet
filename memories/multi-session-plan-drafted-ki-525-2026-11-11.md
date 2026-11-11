---
name: multi-session-plan-drafted-ki-525-2026-11-11
description: "project 2026-11-11 — KI-525 FIXED+CLOSED, multi-session plan COMPLETED, v0.6.28 DELIVERED to GitHub (anonymous feed read-back names 0.6.28 + cli sha256 match); the KI burndown then took the open ledger 5→1 (KI-513/466/480/456 fixed+closed, KI-252 clustering plan drafted, stays open); dev-decisions branch merged WHOLE to main + pushed (01f1377); owner ordered \"merge the whole thing, rebuild the app, make the github cut\" → v0.6.29 DELIVERED to GitHub (anonymous feed read-back names 0.6.29 + cli sha256 2671dc0…"
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

The owner ran a KI-ledger burndown session from the Tauri app, then opened
a second session (obscura's browser use vs fleet's) to test two active
sessions. The KI session showed task failures and the second vanished
from the rail.

**The find — KI-525 (filed, open).** The face is single-run: ONE serial
agent thread (`src/gui/bridge.rs:13063`, `while let Some(command) =
commands.recv().await`) with a single `driving` field, while its STATE is
already a per-session table (`GuiState.sessions`). A second run attempted
while the first drove, plus a face restart, desynced the two: `/api/state`
read `running=true` on an EMPTY attached chat (`363d3259`, one start line,
no transcript) while `driving` still named the dead KI run (`35fdca4b`,
"let's work on the open KI ledger items", log ends mid-run with no `end`
record). Nothing was executing, but the stuck flag made `dispatch` refuse
every send ("a run is already in flight", bridge.rs:5383) — a phantom wedge.
The second session never executed into a log (invisible); the first's
interrupted run left its task card at in_progress/pending (the "failures").

**Recovery.** `launchctl kickstart -k gui/$(id -u)/com.fleet.gui` WORKED
this day (new pid 42400, `running=false`, `driving=None`); the KI log is
intact on disk and resumable. The relay DNS failure in the same log
(fleet.opsh.io / fleetbot.opsh.io not resolving) is UNRELATED — the relay
is a separate spawned task (mod.rs:206, remote/relay.rs:399) the local run
path never dials; the owner corrected my framing of it as a cause.

**The plan — docs/plans/2026-11-11-multi-session.md (status ACTIVE).** One
path, two stages: Stage 1 makes single-run honest (restart reconstructs
running=false unless truly live; a named refusal card, never a phantom;
session lifecycle strands no flag — closes KI-525 + KI-524), ships as a
beta cut alone. Stage 2 lifts the serial loop to per-session run loops
(the state table already exists), admission-gated by `contract::admitting`
(src/contract.rs:74). Six criteria C0–C5. **plan-gate PASS via drex (6/6),
arch-gate PASS with 13 UNDOCUMENTED conventions gaps (the new area, zero
conflicts)** — confirming [[board-state-closeout-2026-11-10]]'s "only the
jev tail is dead".

**APPROVED 2026-11-11 — the owner's goal: "implement plan as designed, make
sure that the application is built and cut makes it to github."** Plan
flipped draft→active; implementation DELEGATED to a background general-purpose
agent (per-session run-loop extraction + admission + polite card + tests +
four-leg gate); the parent retains verification (re-read diff, re-run gate)
and the cut. Nothing committed yet at delegation time.

**Implementation design facts (verified in source before delegating):**
`Agent<'a>` (src/agent.rs) BORROWS client/log/progress, so the whole Run-arm
setup+execution must move into a spawned task that OWNS its locals; the route
maps (routes/route_pins/judge_tasks) become Arc<Mutex<HashMap>>, never held
across an await. The runtime is `new_multi_thread().worker_threads(1)` — async
concurrency at await points suffices for I/O-bound model calls. `record()`
(bridge.rs:12309) already clears running/driving on finished/failed events;
the wedge is purely a queued run the serial thread never reaches. Per-run disk
admission ALREADY wired in agent.rs ~3665–3779 (admit_once) — leave intact.
C3 design: pure `contract::admits_concurrent(reading, in_flight)` scaling
run_budget_kib by (in_flight+1) through footprint::admit (strict headroom >
estimate); Reading has memory_used_share, MEMORY_CEILING_KIB=2GiB,
PROCESS_CEILING=80.

**Release readiness for v0.6.28 (verified 2026-11-11):** gh authenticated
(adelvillar1), updater signing key present at /tmp/fleet-updater-key (no vault
passphrase needed this cut), cargo-tauri 2.12.4, 77 GiB free. Version bumps:
Cargo.toml:3, src-tauri/Cargo.toml:3, src-tauri/tauri.conf.json:4, Cargo.lock
(2 hits). Tree state: KI-524/KI-492 fix COMMITTED by a co-tenant at 7b15f13;
src/health.rs carries a COMPLETE co-tenant KI-513 fix (merged_over/carried,
wired into probe_once, tested) — rides the cut as its own commit, do not
disturb. release.sh refuses a dirty tree in both modes.

**The owner's governing steer (2026-11-11):** concurrency is allowed as
long as the environment can handle it — the ceiling is the machine's
capacity via the resource contract, NOT a fixed pool size; when too
constrained, fleet POLITELY says so with a named card offering
reconsider-or-queue, never a silent queue or terse refusal. Folded into
the Approach + C3. See [[the-resource-contract-admit-work-settle-curtail]].

Triage lessons: session logs store the task in `user.content` (not
`text`); titles come from the first message, so MANY sessions are titled
"warmup" (the owner opens almost every session with one) — identify a
session by its content, not its title. The face is the INSTALLED binary
([[the-served-face-is-the-installed-copy]]); the uncommitted KI-524 fix in
the tree was NOT live.

**DELIVERED STATE (2026-11-11, later).** Implementation landed (delegated
agent, verified by the parent's OWN gate, not its report): four-leg gate
FMT/CLIPPY/BUILD/TEST all 0 (2,389 passed), every C0–C5 pin test ran green,
face harness 1,108 checks / 140 cases exit 0. Deviation recorded: each admitted
run gets its own thread + current-thread runtime (NOT tokio::spawn) because
`Agent<'a>` is `!Send`; per-run event forwarders keep transcripts isolated.
Plan flipped COMPLETED (all boxes ticked + a Verification-evidence section);
KI-525 CLOSED in the ledger. Commits (explicit paths): 2139bc3 feature,
c2ee2e8 KI-513 health merge, 67e0c59 armada direction note, 244f856 churn,
ff13ebe 0.6.28 bump+changelog, 954e91a the test fix, a153a84 churn = HEAD/tag.

**The gate caught a real flake (first cut REFUSED, nothing tagged).** The C2
test `two_concurrent_runs_grow_their_own_logs_and_stay_isolated` drove
UNPINNED runs, so each spawned the sys1 lane to judge its route — fast alone
(passed 3×, ~1.5s) but slow under the full suite's parallel load; in the
release worktree run one stalled before its `routed` event and the 12s wait
expired. FIX: pin both runs to the mock's `stand-in/auto` via
`bridge.run_with(.., Mode::Ask, Some("stand-in/auto"), None)` — a
provider/model pin skips the judgment (route::select's Pinned arm), so the
test pins concurrency, not routing. Mode::Ask not Yes (full_access_floor
refuses Yes with no folder). Verified under the REAL condition: full lib
suite 2,176/0 in 41.8s. See [[a-load-sensitive-test-is-a-defect-in-the-code]]
and [[house-load-sensitive-tests]] — here the fix was isolating the test from
an external lane's timing, legitimate because the test's subject is
concurrency. NOTE: my working-tree gate's PATH lacked node (~/.local/bin) so
the face harness SKIPPED there; the release gate's PATH has node so it RUNS —
always run the harness explicitly to prove C5.

**v0.6.28 DELIVERED + verified (2026-11-11).** `release.sh tag v0.6.28`
re-gated HEAD on a detached worktree (green), settle clean (1.08 GiB / 3.44
budget), tag pushed, GitHub release created (isDraft:false). `release.sh
assets v0.6.28` completed: all six assets on the release (dmg
fleet_0.6.28_aarch64.dmg, CLI tarball, raw binary, fleet.app.tar.gz + .sig,
latest.json); the ANONYMOUS feed read-back proves delivery — "the published
feed names 0.6.28; the cli row serves the raw binary, sha256 810042e37e46".
Two honest non-failures: the signer warned "Signing without an app version"
(harmless — fleet's updater config has NO requireSignedVersion); the assets
settle recorded a RUNAWAY (0.65 GiB used vs 0.31 budget) — a calibration row
the contract learns from, not a cut failure. The cut correctly refuses a red
gate — never tag broken code.

**KI BURNDOWN EXECUTED — open ledger 5→1 (owner takeover, 2026-11-11).**
Owner: "would you mind taking over the ki burndown?" then "/steer let's wait
the cut and then merge" (no source edits until the assets cut landed). Two
owner rulings via AskUserQuestion: KI-466 → task strings name the FILE (fact
stays dotless); KI-252 → draft a clustering plan. Results:
- KI-513 CLOSED (already fixed c2ee2e8, shipped 0.6.28).
- KI-466 FIXED+CLOSED: cron.rs seed_task now instructs the dotless fact name
  corpus-{project}-<YYYY-MM-DD> and explains the substrate writes the file
  ...md (the ignore-rule spelling); pin strengthened in
  the_manifest_task_names_the_route_the_driver_has; cron 54 green.
- KI-480 FIXED+CLOSED: session_recall is_house_word (closed set of bare
  greetings) + task_text falls past house words to the first substantive
  owner message; digest reads every owner message + compaction summary ahead
  of tool calls (recall finds a conversation by content words); the rail
  title falls to lane_summary when the opener is a house word. Pins
  a_house_word_opener_is_fallen_past_and_the_digest_reads_the_conversation +
  house_words_are_a_closed_set_of_bare_greetings; session_recall 16 + bridge
  94 green. The last-owner-message rung (no lane_summary case) is the recorded
  further refinement.
- KI-456 FIXED+CLOSED in dev-decisions (01f1377): _parse_plan_criteria scoped
  to the Acceptance criteria heading (split at ## , skip fenced spans) like
  plan_check.py; the plan that shifted parses 14 (was 19), first the author's
  C0; TestPlanGateCriteriaAreScoped added, full selftest 115 green; D-002
  closed there.
- KI-252 STAYS OPEN pointing at docs/plans/2026-11-11-deck-semantic-clustering.md
  (status draft): the unbuilt remainder is semantic clustering of long sources
  into infographic blocks; pure cosine agglomerative grouping over the sem1
  HttpEmbedder, bounded by deck Bounds, measurement-gated default-off.
Ledger well-formed (525 rows, 1 open). Fleet burndown changes: cron.rs,
session_recall.rs, bridge.rs (+ ledger + clustering plan). A fmt --check
failure on the new code was fixed with cargo fmt --all; the four-leg gate was
re-run clean on the formatted code.

**OWNER: "merge the whole thing, rebuild the app, make the github cut"
(2026-11-11).** dev-decisions branch fix/plan-gate-truncation-and-criteria-index
merged WHOLE (fast-forward, carrying co-tenant D-001 + my D-002) to main and
PUSHED (57d3ad0..01f1377 origin/main). Fleet: commit the burndown, bump to
0.6.29 (CHANGELOG section staged: KI-480 naming/recall, KI-466 dotless fact,
KI-456 plan-gate), then release.sh tag v0.6.29 + assets + anonymous feed
read-back. The 0.6.29 cut is IN FLIGHT at this record (gate re-running).

**MERGED + CLEAN (owner: "make sure we merge and leave the tree clean",
2026-11-11).** Burndown committed (9fcf182 fixes+ledger+clustering plan,
6cac8ef 0.6.29 bump+changelog, c16bb6f churn = HEAD); four-leg gate green on
the formatted code (FMT/CLIPPY/BUILD/TEST all 0); lock verified. The release
tag pushes ONLY the tag ref, so master was 10 commits ahead of origin —
`git push origin master` landed (2f27fa8..c16bb6f), remote master == local ==
c16bb6f, nothing unpushed, tree clean. dev-decisions main == origin/main ==
01f1377, clean (co-tenant system.drawio/.svg left untracked). The 0.6.29 tag
cut was still gating HEAD at this record; assets + feed read-back next, then
re-commit any final memory churn so the tree finishes clean. See
[[the-release-is-the-owner-s-no-op]] (the tag-pushes-only-the-tag lesson).

**v0.6.29 DELIVERED + verified; recap landed; trees clean (2026-11-11, close).**
release.sh tag v0.6.29 re-gated HEAD c16bb6f green (settle RUNAWAY 4.55 GiB /
3.29 budget — cold gate lane, a calibration row not a failure), tag pushed,
release created. release.sh assets v0.6.29 completed: all six assets on the
release; ANONYMOUS feed read-back "the published feed names 0.6.29; the cli
row serves the raw binary, sha256 2671dc0ced51"; a fresh curl of the published
latest.json confirms version 0.6.29 (assets settle runaway 0.64/0.31 again,
recorded). Recap docs/recaps/2026-11-11-multi-session-and-burndown.md committed
b656acd + pushed (c16bb6f..b656acd origin/master); final tree CLEAN. Both cuts'
settles outran their budgets — worth a look if it keeps happening. The session
then turned to the cross-harness memory PULL (owner: build now before armada) —
see [[memory-pull-from-other-harnesses-idea-2026-11-11]] (plan active, build
delegated).

Related: [[fleet-gui-is-a-launchagent]], [[face-api-session-in-query-string]],
[[runs-refuse-the-filesystem-with-no-folder]],
[[armada-multi-install-c2-idea-2026-11-11]], [[the-release-is-the-owners-no-op]].
