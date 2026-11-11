# Changelog

All notable changes to fleet land here, one section per release, written at
cut time. The repo was born 2026-10-12; this changelog starts at v0.4.0 with
the first release — everything before it is `git log` (492 commits, each one
through the local gate).

**The beta epoch ended with 0.5.1** — the owner retired the suffix when
asking for diligent version numbering. From here: every user-facing change
bumps the third segment as its own release commit, the three declarations
(`Cargo.toml`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`) and the
lock move together, a section lands in this file, and the running app
answers `GET /api/version` so the owner asks the face what it is.

## v0.6.27 — 2026-11-11

The resource contract's numbers derive, and a stale install can no longer hide.

- **The contract's numbers come from the machine** (C10/C11/C12 of
  `docs/plans/2026-11-09-the-resource-contract.md`). `src/footprint_history.rs`
  is the one reader of the footprint history: volume, the standing free share
  over the last 30 readings, its spread, the fill rate in days, memory pressure
  and laptop/server from the machine's own battery probe. The reserve, the turn
  cadence, the per-run budget, the upkeep interval and a run's admitting prior
  all derive from that profile plus the owner's declaration, so no number stays
  a code constant once a profile exists.
- **The appetite is declared, and it governs everything.** `appetite`
  (`tight`/`balanced`/`roomy`, or `steady`) and `footprint.reserve_gib` are
  settings at last — the first was read by the contract but refused by
  `fleet settings` since it landed. One grade is in force, so a declaration
  tightens the reserve, the cadence, the run budget and the upkeep together.
  `fleet settings unset <key>` is a CLI verb for the first time.
- **The evidence rides beside every number.** `environment_clause()` prints the
  inference and what produced it — *"reserve 18 GiB of 460 GiB (inferred; laptop
  25% free for 2 record(s) ±0 pt)"* — on `fleet footprint`, `fleet doctor`, the
  run loop's admission and refusal, and the JSON document.
- **The upkeep cadence is consumed, not printed**: `scripts/daily-maintenance.sh`
  reads `upkeep_days` as the build lanes' retention window, so a machine filling
  fast revisits its caches sooner. `scripts/footprint.sh --record` now writes
  memory into the same history row.
- **A stale install is a named FAIL.** `fleet doctor` gains a `binary` line that
  compares the bytes of the built binary against the installed copy — the check
  `GET /api/version` cannot make, since a rebuild keeps the same version — and
  names the remedy (`fleet reload --from <build>`). It found the owner's own
  console serving a thirteen-hour-old binary at this cut.
- Fixed en route: `reload::source_beside_home` walked two parents and searched
  `/Users/Projects`, so the build-beside-the-home fallback had never found the
  checkout (KI-521); `contract::env_from_history` dropped the fill rate, so every
  run's upkeep read as "not filling" (KI-519); a declared appetite moved the
  reserve while leaving the cadence and budgets on the inferred grade (KI-518);
  and `appetite` was unreachable from every settings surface (KI-517).

## v0.6.26 — 2026-11-10

Experimentation over refusal (the owner's rule 8).

- **The sandbox-experiment gate**: a destructive-flagged bash command
  in a git repo runs in an isolated clone first — the working tree
  carried in (tracked diff applied, untracked copied) — and the
  outcome (status + diff) rides the tool result. The real repo is
  untouched until the ordinary approval gate: the owner approves the
  outcome, not the guess. Bounded (repo size, timeout, output); a
  skipped experiment names itself; `experiments.destructive` (default
  on) is the off-switch.

## v0.6.25 — 2026-11-10

The grounded-selection arc lands its measured half.

- **The hosted generation seam** (`src/generate.rs`, blend1's
  `generate` tool): compaction folds, conversation titles, and archive
  summaries ride the hosted small model (LFM3-1.7B, probed) with
  today's paths as named fallbacks; the face renders `lane_summary`.
  Skill-proposal bodies and the tools-advise card narrate through the
  same lane; deterministic inputs stay byte-identical.
- **The repair leg for every tool** (`prepare_call_checked`): a
  validation-failed call — any tool — gets one re-author through the
  hosted lane before the error-result loop, behind the tool's own
  schema as the guard. Measured basis: the verifier's 0.983 separation
  AUC with 60/60 corruption rejection.
- **The destructive head joins the gate**: an unlisted bash command
  asks a yes/no head through an inline `task_spec`; a `yes` annotates
  the call so the ordinary gate asks. Measured 1/1 destructive recall
  with 76/77 safe precision where the list catches 0/1.
- **Vision QC**: gen1's rendered images grade through sys1's
  liquid-only `vision_qc` task, verdicts as calib rows naming the
  artifact.
- **`fleet tools audit --all`**: the archive-wide measurement — the
  baseline the selection program's criteria compare against.
- **KI-515 closed** (the blend1 corpus tests read the 5-value
  contract); **KI-516 refined** (the coherence test's lock window
  closed; the flood diagnosed — the override scoping is the recorded
  fix).
- **The rejections recorded**: selection replacement (eight shapes
  measured, the meta-failure named), score fan-out at catalog size,
  LFM3 as selector, cross-vendor score transplant, hard narrowing —
  each with its measurement in `docs/features/selection.md`.

## v0.6.24 — 2026-11-10

The auto-update channel's supply gets a guard.

- **The health loop names an uncut build** (KI-512). The Tauri updater
  reads GitHub's `latest.json` and nothing else, so a version installed
  locally but never cut through `scripts/release.sh` is invisible to
  every installed app — 0.6.22 and 0.6.23 both shipped that way before
  they were cut retroactively. The loop's slow tier now carries a
  **release** line: a build whose version has no release tag reads
  DOWN with the cut as its repair, and a tagged build whose published
  feed names an older version reads DOWN as half-cut (the tag exists,
  the assets never published). The face paints it beside the other
  services, so the gap is visible the moment it exists.
- **The changelog's sections are in descending version order** — a
  section anchored below parallel cuts had put 0.6.22 under 0.6.19 and
  made the release script refuse its own notes.
- **The release gate caught what two green working trees had hidden**:
  the health tests shared one temp home keyed by process id, so a
  parallel test's fixture-delete raced another's probe. Each test owns
  its labeled home now; the gate's detached worktree is why the race
  surfaced at all.

## v0.6.23 — 2026-11-10

Two live-surface defects, found by the owner's own eyes on the
delivered face.

- **The router page stopped blanking mid-run** (KI-510). The live
  decision card's facts edit had orphaned its rows array; the paint
  threw on the first `routed` event and the board died after clearing.
  Idle read fine — blank exactly while work was being routed. The
  rows append to their list again, pinned by the harness case that
  paints a routed event and asserts the facts, the execution head,
  and the walk.
- **The workspace-history pane names what it lost, and the act**
  (KI-511). The Docker reset removed every registered project's Store
  volume; the read named it, the pane didn't. It now renders a
  "stores lost to the Docker reset" card — each project, its cause,
  the script noise trimmed — and each line carries a **re-register**
  button: the bridge re-imports the folder (the history restarts
  empty), drops the rail cache, and names the outcome.

## v0.6.22 — 2026-11-10

The failsafe: the chain restores fleet to working order, never refuses
(the failsafe-restore plan, phases A through D).

- **The health loop** — the app probes the critical services on a beat
  (layerfs's worker port, each vendored lane's presence, docker's
  daemon, the API's key), `/api/state` carries the verdicts, and the
  face paints a card naming every down service beside its repair
  verb. A transition announces itself; the face repaints without
  waiting for its poll.
- **Degradation over refusal** — a delegation whose LayerFS workspace
  cannot open runs on the main session's model and route with the
  plain launch folder; a child whose route cannot be prepared rides
  the parent's route; a routing config that cannot load degrades the
  run to the compiled-in declared order. Every degradation is named
  in the row, the result, and the transcript — never silent, never a
  refusal.
- **The repair note** — a run whose provisioned substrate is named
  down carries the repair on its first request: the service, the
  cause, and the verb, logged as `SystemNote` so the request stays
  reconstructable. The main session repairs through its own bash and
  the standing approval gate; an exhausted chain names every
  candidate's cause and its repair — and no connectivity with no
  local model configured is named as the one unrecoverable failure.
- **The local tail and `fleet recover`** — routing.json's
  `local_tail: "provider/model"` puts a configured local model behind
  the deepseek tail as the chain's last rung; `fleet recover`
  restores every settings file whose current form is broken from its
  last-known-good stamp (every settings write stamps one) and names
  the probes' fresh verdicts.

## v0.6.21 — 2026-11-10

Fleet stops being a good citizen and starts being a responsible one: work is
admitted against the room that exists, settled against what it declared, and
curtailed when it outgrows either.

- **`fleet footprint`** reports what this process tree holds right now — memory
  and process count against declared ceilings — plus the environment line: the
  appetite the volume's own history implies and the reserve that follows from
  it, named with its source (`roomy — reserve 18 GiB of 460 GiB (inferred; 1
  record(s), standing 25% free)`). `--json` publishes the same facts as one
  document for a script to read.
- **The contract's four beats.** `--class <work>` adds that class's estimate,
  headroom and admission; `--estimate` reports what the class's own settlements
  say it costs; `--record` lands the admission; `--settle <before_kib>` records
  the settlement and exits non-zero on a runaway. An estimate exactly equal to
  the headroom is refused: work costing exactly what is spare eats the reserve.
- **A run in a folder is governed.** It admits before its first turn, measures
  at the turn boundary every fifth turn, and stops with the numbers in the
  transcript rather than after the disk is full. A file-less chat is untouched.
- **A deterministic breach is a pause**, the owner's ruling: the run holds and
  puts the numbers in front of you, continuing only on a yes. Nobody present —
  an unattended job, a headless batch — answers no, and `--yes` is not standing
  consent for this. A yes buys one more run's budget of room below the reserve
  and never compounds; it lowers the floor, it does not excuse the budget.
- **The abnormal reading asks sys1 once**, bounded by the same daily cap the
  routing judge keeps. `runaway` curtails and raises nothing; `expected` raises
  that run's budget so the identical shape does not ask twice; a lane that did
  not answer reads as `expected` and records itself, never a silent stop.
- **The estimates learn.** A work class's admitted cost is the median of its
  last ten settlements once three exist, and a judged verdict is graded by the
  outcome that followed it. A settlement of exactly zero measured nothing and
  teaches nothing.
- **`fleet doctor` grades the disk against the same reserve** the report
  derives, so the two surfaces cannot disagree about the number that decides a
  refusal: `disk ok 107 GiB free (reserve 18 GiB)`.
- **`scripts/release.sh` asks the contract** in both modes instead of a
  hardcoded floor — admitting before the build and settling after the work,
  through the binary rather than a re-derived shell number, with the declared
  floor as the named fallback when no binary on the tree answers.
- The numbers — 2 GiB memory, 80 processes, 20 GiB floor, reserve share tight
  12% / balanced 8% / roomy 4%, the per-class estimates — are the owner's,
  settled 2026-11-10.

Three defects were found by driving the arms rather than reading them, all
fixed in this release: `release.sh` discarded its own refusal (a shell idiom
that threw the JSON away exactly when the verdict was `refused`), the reserve
was sized from the history's disk rather than the one measured, and a
zero-delta settlement taught a zero estimate.

## v0.6.20 — 2026-11-09

The build caches get a floor, a cap and a line in `fleet doctor`.

- **`fleet doctor` reports the disk**, judged against a 20 GiB floor: below it
  the line says what dies (a build mid-link) and who holds the space (the
  `/private/tmp` build lanes and `~/.fleet/checkpoints`). The check is
  hermetic and pinned; the live answer on this machine is `113 GiB free`.
- **`scripts/release.sh` preflights the volume** before either mode builds: a
  volume under `FLEET_DISK_FLOOR_GIB` (20) refuses the cut with the numbers,
  and the gate's own lane is cleaned when it passes
  `FLEET_GATE_TARGET_MAX_GIB` (20) instead of growing without bound.
- **`scripts/daily-maintenance.sh` caps a lane by size as well as by age** — a
  lane that grew to tens of gigabytes inside the warm window is cleaned, not
  shielded by its warmth (the 69 GiB gate lane that took the drive to 1.6 GiB
  free, KI-502).

## v0.6.19 — 2026-11-09

The same content as v0.6.18, cut again because v0.6.18's assets never
uploaded: the release was tagged while a co-tenant's commit landed on the
branch, the assets build refused to run off-tag, and an empty release held
`releases/latest` until it was marked a prerelease. v0.6.19 carries the
section below, and the changelog keeps 0.6.18 as the record.

## v0.6.18 — 2026-11-09

One composer, the app's list column, the artifact in the centre, its
properties in the pane.

- **The desk and learning borrow the conversation's composer.** The bespoke
  composer each surface drew is gone: the field, the run bar, the queue and
  the Stop are the chat's own element, moved into the surface's column and
  back when the route leaves. Its focus changes per surface — the desk's
  words iterate on the open deliverable or open the brief with the claim
  already written; learning's words answer into the lesson's session.
- **The list column is the app's own.** The second projects rail is gone:
  the desk lists deliverables and learning lists lessons on the same work
  column sessions and huddles ride — same head, rows, fold and counts. A
  desk row carries the deliverable's real state (`1 refusal stands`,
  `all five stages done`, `not started`), and a project the server could not
  parse names itself in the list.
- **The centre is the artifact**, not just its conversation: the desk shows
  the deliverable's cards (stage rail, claims, coverage, outline, review, end
  band) and then its thread; learning shows the lesson's section and then its
  thread. With nothing open, both show the surface's welcome in the
  conversation's own empty-state component.
- **The right pane is properties and settings** — project, shape, sources,
  the end gate, the role, the sections — in the pane's card, facts and ghost
  button idiom. It exists only while an artifact is open; otherwise the tab is
  absent and the pane falls back to the run.
- **The composer's fields collapse by its own width** (a container query):
  a work surface's column is narrower than the conversation's, and the run
  bar drops its words for icons instead of overflowing.

## v0.6.17 — 2026-11-09

The composer moves to the centre, and the pane carries the work.

- **The layout both surfaces now use** (the owner's correction): the
  centre column is the artifact's **conversation and composer**, and the
  right-hand column is the app's own **collapsible pane** — the same one
  sessions, design and huddles carry — with a new leading tab,
  **`the work`**, whose content is whatever the route is about: the
  desk's deliverable (or its shelf and brief), learning's plan, graph,
  practice and history. The projects rail stays on the left.
- The tab appears and hides with the route (`syncPaneContext`): a
  surface with no context hides it, and a pane showing it on a route
  that has none falls back to the run tab, so no tab is ever a dead end.
- Learning's pane stopped being a section rail: the pane is a plain
  board with the same shell as the desk.
- The message list is bounded (300 px) so the composer lands in view,
  and only one pane body shows at a time.

## v0.6.16 — 2026-11-09

The shells span the board.

- 0.6.15's two shells landed in one column of the board's own twelve-
  column grid, so both panes squeezed to a twelfth of the width: the
  sections stacked and the thread overlapped the work. Found by the
  visual read of the shipped surface. `.pw-shell` and `.pw-we` now span
  every column (`grid-column: 1 / -1`), and the desk and learning read
  as three columns — the projects rail, the work, the thread.

## v0.6.15 — 2026-11-09

The lesson's thread names its project.

- 0.6.14's learning document returned each lesson's project in the
  history rows but not on the lesson the pane shows, so the thread's
  project chip read `unbound` while the rail had the lesson under its
  folder. The read now carries it on the lesson too (its session's own
  working directory), and the chip and the rail cannot disagree. Found
  by the release's own live read.

## v0.6.14 — 2026-11-09

The desk and learning, organised by project and worked in a thread.

- **KI-501 fixed.** Both surfaces are one shell now: **the projects rail on
  the left, the artifact's work in the middle, its thread on the right.**
  The rail reads one `GET /api/projects` — the desk's bound deliverables
  and the lessons' own working directories, with an unbound bucket — so a
  project is one name and one count wherever it is read.
- **The desk iterates in a conversation.** A deliverable's thread is a
  session of kind `desk`, born on the first iteration and seeded with the
  brief; `POST /api/desk/<slug>/chat` runs the ordinary run path with the
  deliverable's project as its folder. The loop's cards stay where they
  were, beside the thread.
- **Learning answers in the lesson.** The thread is the lesson session's
  own log; the composer sends into that session (`?session=`), its role
  chip is the role the run rides, and the practice chip starts a practice
  beside it. The four sections moved to chips in the work column now that
  the pane's rail is the projects rail.
- The composer's chips act: the project selects it in the rail, the
  artifact re-reads, `sources` opens the brief's source list, `review`
  shows the end gate, `role` flips, `practice` starts one. An empty rail
  offers the act that fills it, and a refusal paints beside the composer.
- Pinned by `PWS the desk is projects, work and a thread` and `PWS
  learning is projects, the lesson and its thread`: the rail lists,
  expands, opens, the thread replays, the composer posts the right run,
  and every chip changes something. 1076 harness checks / 134 cases.

## v0.6.13 — 2026-11-09

The press answers at once.

- A decision's read is not instant — the inbox document probes the
  doctor — so a press that waited for the reply changed nothing on
  screen for seconds and read as a dead button. `dismiss`, `archive`,
  `restore` and `undismiss` now paint the row's fate in the same tick
  (the row goes, or comes back) and the hint says `working — dismissing`
  while the reply rides; the reply reconciles, and a refusal rolls the
  paint back and names its reason in the pane. Red-proven: with the
  optimistic paint removed, `IBC the dismiss answers before the reply
  lands` fails with the idle hint still showing.
- The ages read as words, not "now ago": `dismissed just now`,
  `archived 2h ago`.

## v0.6.12 — 2026-11-09

A dismissal is never a trap.

*Release and tag deleted 2026-11-09: 0.6.13 carries the same content plus the
instant press, and nothing depends on the version being installable.*

- **v0.6.11's cut was burned** (release and tag deleted 2026-11-09): the version bump was piped through
  `head`, SIGPIPE stopped the script after two sites, and the tree went
  to the tag with `tauri.conf.json` still at 0.6.10 while the two
  manifests said 0.6.11. The assets mode's three-way check refused
  (the dmg came out named 0.6.10), the release exists with no assets,
  and it briefly held `releases/latest` — so the feed 404'd until the
  release was marked a prerelease and `latest` fell back to 0.6.10.
  This cut is the same content with the version sites repaired.

- The read now carries the **dismissed** rows (their day and whether
  their fact is still waiting), and the pane offers them behind one
  quiet toggle — `dismissed · N` — with a **bring back** on each. The
  `undismiss` op existed from 0.6.8 but nothing in the interface could
  reach it: a row dismissed by mistake had no way home until its fact
  changed. Pinned by `IBC a dismissed row can come back` (the toggle
  counts, the row names itself, the press posts the undismiss and the
  row returns).

## v0.6.10 — 2026-11-09

The inbox's group headers say their words again.

- **KI-499 fixed.** Every group header read as a type name —
  `[object HTMLSpanElement]` — where "setting fleet up" and "the
  machinery needs a decision" belonged: the header was built with
  `node(name, className, text)` and handed a list of elements, which the
  helper writes as text. Both heads are built by appending now, and the
  harness pins all five (`IB the group heads say their words, not a type
  name`); with the old shape restored it fails with `[object Object]`
  for every head.
- Found by the redesigned inbox's own live read, which is the point of
  reading the surface after shipping it.

## v0.6.9 — 2026-11-09

The archive leaves the queue.

- **The archived row stopped waiting.** 0.6.8's read filtered only the
  dismissed decisions, so an archived row appeared in the Archived segment
  *and* stayed in the waiting queue — found by the release's own live
  check (`POST /api/inbox {"op":"archive"}` answered with the row still
  in `total`). The filter now knows both decisions, and the archive's
  "the fact is still waiting" flag is read before the decisions hide the
  rows, so an archived row reads `live: true` while its fact is there.
- Pinned by `an_archived_row_leaves_the_waiting_queue`, which drives the
  bridge's own `inbox_op` and asserts the queue empties, the archive
  holds the row, and a restore brings it back.

## v0.6.8 — 2026-11-09

The inbox can be worked.

- **KI-496 fixed.** Every onboarding row's only button set the hash to
  `#/chat` — the conversation the owner was already reading — so the
  setup press did nothing visible. Each ladder step now lands on its own
  surface through one table (`vault` → `#/vault`, `github` → the settings
  account card, the rest → `#/settings`), and *not now* is a dismiss.
- **KI-497 fixed.** The inbox is a derived view, so a row could only
  leave by its fact resolving: there was no dismiss and no archive.
  `~/.fleet/inbox.json` now keeps the owner's decisions, keyed by the
  row (`class:id`) with the fact's own signature — a dismissed row comes
  back when its fact changes, and an archived row keeps its day and stays
  restorable, even after its fact resolves.
- The pane gains the **Waiting / Archived** segments, class filters with
  counts, a **context menu on every row** (`open`, `dismiss`, `archive`;
  a seat's menu carries only `open` — a seat holds a run and cannot be
  hidden), and a keyboard (`j`/`k` move, `enter` runs the row's own verb,
  `d` opens its menu).
- A refused decision now paints its reason **in the inbox pane**, beside
  the row that asked — never only in the transcript two panes away.
- The settings paint no longer throws on a document without its lists
  (a read that did not land now shows the waiting card).

## v0.6.7 — 2026-11-09

The send button works with a message queued.

- **KI-495 fixed.** Two shapes, one control. An idle press with
  nothing typed and a message waiting ran the send path, which tried to
  queue the empty composer and returned — no run, no queue change, no
  line anywhere, so a queue that outlived its run could not be moved at
  all. And a face that attached to a running chat (a reload, a
  conversation opened mid-run) learns `running` from the state read but
  has no process card until a process event lands: the Stop it showed
  returned silently, a dead control under a live run. The press now
  sends the queue's head when the face is idle with nothing typed (a
  run working in another conversation still keeps the queue waiting),
  and Stop fires whenever the face believes a run is in flight — the
  process card only decides what the reply paints.
- Red-proven both ways: with the change reverted, the two new harness
  cases fail with no `/api/run` and no `/api/stop` posted and the
  queue still reading `queued · 1`.

## v0.6.6 — 2026-11-09

The release the updater installs.

- **No behaviour change from 0.6.5.** This cut exists to be the first
  release a fixed channel installs: 0.6.5 carries the updater's fix and
  was installed by hand (its predecessor's compiled-in ACL could not
  reach the updater), and 0.6.6 is what the 0.6.5 shell's About press
  and `fleet update` on a 0.6.5 copy install — the end-to-end proof the
  two channels work.
- The release also carries the feed's own read-back (0.6.5's cut): the
  published `latest.json` names the raw CLI artifact and the served
  bytes hash to its signature.

## v0.6.5 — 2026-11-09

The updater installs.

- **KI-486 fixed.** The CLI channel wrote the release's gzip tarball over
  the running binary: the feed's cli row hashed that tarball, so every
  assertion passed and the swap destroyed what it installed. The release
  now ships the channel's artifact as a raw binary
  (`fleet-aarch64-apple-darwin`) and the feed names it; before the
  rename the staged file must **run and answer `--version` with the
  feed's version**, so a tarball, a truncation or a foreign build refuses
  and leaves the tree untouched. The replaced binary is kept as
  `<name>.prev`, the receipt names the version the feed installed, and a
  bundled executable refuses by name — the `.app` updates through its
  own window.
- **KI-487 fixed.** The shell's update press never reached the network:
  the window loads a loopback `http` origin, which Tauri classifies as
  remote, while the capability compiled local-only. The capability now
  names that origin as a remote context and grants the app's own
  `restart_app`; the shell binds its listener once and hands it to the
  server, so the port the window trusts is never left free for another
  process to take.
- **KI-488 fixed.** The About pane painted "nothing newer" before any
  feed read, and kept it when the read failed. It now says the feed has
  not been asked until the check answers, and a failed check paints the
  refusal beside the press.
- **KI-489 fixed.** `dangerousInsecureTransportProtocol` is gone: the
  updater's transport is HTTPS and nothing else, per the owner's rule
  that an update is fetched from GitHub, never from the local machine.
- The cut reads its own feed back anonymously and hashes the served
  artifact against the feed's signature — an upload is not a
  publication.
- **KI-490 fixed.** `scripts/bump-version.sh` printed "moved: 2" over a
  `Cargo.lock` mutation it never wrote, so the lock kept the old
  version and the next cargo run agreed with it.

## v0.6.4 — 2026-11-09

A session that was interrupted stays interrupted on the screen.

- **KI-485 fixed.** The owner returning to sessions saw bash tool rows
  ticking "running · Ns…" forever and run cards pulsing "in flight":
  nothing was running — the PAGE painted it. A log cut off at a bare
  `tool_call` (every stop, crash, or face restart leaves one) replays
  with no closing event to pair the row, and `toolRow`'s seconds
  interval never clears without a paired result. The replay now sweeps
  the unpaired calls and settles them with the honest end — "N ·
  interrupted" — the same shape a live refusal paints. The paired rows
  are untouched; the run card's own interrupted closure (KI-423) was
  already in place beside them.
- Red-proven: with the sweep removed, the pin fails with the row back
  at "running · 0s" and the phantom text intact.

## v0.6.3 — 2026-11-09

The write guard stops refusing what the project quotes.

- **KI-484 fixed.** The write-time guard polices each word a write
  coins, but it read a markdown write's multi-word backtick span — a
  tool's verbatim stdout quoted for the record — as one coined
  identifier per word, and refused the recap that documented the 0.6.2
  work (its `Signature and comment signature verified` line died
  against the established `signature`). A multi-word inline span is now
  exempt at the guard's own pass: it is one quoted thing, not one
  coinage per word. The boundary holds: a single-word span still
  polices, the same span in a Rust string still polices, fenced code
  still polices, and the derivation is untouched — spans still carry
  identifier evidence for the spelling convention. The span finder and
  the word splitter are crate-visible and shared with the channel
  reader, so the guard's reader cannot disagree with the vocabulary's.
- Red-proven: with the exemption reverted, the guard pin fails naming
  the same word the 0.6.2 recap was refused for.

## v0.6.2 — 2026-11-08

The update button answers where it is pressed.

- **KI-483 fixed — the About pane's update button works.** The CLI
  channel's apply read `feed.row("cli")`, but the feed parser kept only
  the `platforms` table and release.sh ships the cli row as a top-level
  `cli` object beside it — so every apply answered a 409 the button then
  painted two panes away from where the owner pressed. The parser now
  keeps the cli row (the format's two row shapes both live in the feed:
  the plugin's rows under `platforms`, the CLI's row beside them), and
  the refusal paints into the About pane's own feed answer. Pinned in
  `update::` (revert-proven: the mapping removed fails compilation and
  the dropped row fails at the 0.6.0 message) and in the face harness
  (the 409 names itself in the pane, the button re-arms).
- Honest note: this fix is compiled into >=0.6.2. A 0.6.0 face's button
  stays dead on the CLI channel regardless of the feed; from a 0.6.0
  dmg shell the working hop is the Tauri-native updater.

## v0.6.1 — 2026-11-08

The fold budget followed the workloads' real serving pairs.

- **KI-482 fixed.** The compaction budget derives from the serving pair's
  declared context window, but the compiled table named ollama's copy of
  `deepseek-v4.1-flash` only — while the owner's routing walks `hard`,
  `deep_context` and `frontend` with `opencode`'s copy first. The unknown
  pair floored the budget at 100k, so long sessions folded six times more
  often than a 1M-window model needs (19 folds across the day's two long
  sessions, every one at ~100,0xx tokens). The table now declares every
  copy of the served generations (`opencode`, `token-plan`, their `-free`
  spellings, ~1.05M / 1M to match the same generation's window
  elsewhere), pinned by
  `every_copy_of_a_served_generation_declares_its_window`.
- **release.sh feeds the bare name.** The v0.6.0 cut's `latest.json`
  uploaded under the scratch basename `fleet-latest.json`, so the pinned
  feed URL 404'd until a hand-clobbered upload; the scratch now IS
  `/tmp/latest.json` and the feed leg cannot detach from its name again.

## v0.6.0 — 2026-11-08

The self-updater. The shell now asks the release feed from inside the app
and installs its own next version — 0.6.0 is the last manual install, and
every release after it is a button.

- **The updater channel.** `src/update.rs` fetches the pinned GitHub feed
  (`releases/latest/download/latest.json`), refuses malformed feeds and
  refusals (a refused check is never read as up-to-date), version-gates
  "behind" against "same"/"ahead", verifies the CLI tarball's sha256, and
  stages + atomically renames the new binary in place. Pinned by 8 unit
  tests.
- **The face routes.** `GET /api/update/check` and
  `POST /api/update/apply` on the face's server, carried by the bridge
  (`Bridge::check`, `Bridge::update_apply`), stamping the settings' last
  check.
- **The shell's own updater.** `tauri-plugin-updater` is wired into
  `src-tauri` (pubkey, endpoint, `createUpdaterArtifacts`) with
  `withGlobalTauri` so the plugin's JS API is reachable; the About pane's
  apply button downloads, installs the new bundle and relaunches
  (`restart_app`).
- **The About pane.** A new System group pane: this build, how it was
  installed, the release feed's answer (candidate / refused /
  nothing-newer), the apply button lives here; the settings' `update`
  card points at it.
- **release.sh cuts the feed.** The three-way version refusal (tauri.conf
  ↔ dmg name ↔ bundled literal), the `.app.tar.gz` discovery, the
  `cargo-tauri signer sign` with an empty passphrase, `latest.json`
  generation (the shell row's minisign signature, the CLI row's sha256)
  and its upload alongside the dmg and the tarball.

## v0.5.1 — 2026-11-08

The diligence release, born from the loop it names. The once-daily
21-scenario eval corpus had been firing on the face's every second since
daybreak — 3,143 rows in the calibration ledger, 24,679 temp directories,
a 40 MB day stream — because an *empty* served model counted as a model
switch.

- **KI-481 — the eval corpus fired every second, never daily.** The face
  hands the scheduler `""` as its served model; the guard read a persisted
  empty (trimmed to `None`) as *changed* and "changed" was forever. Empty
  is no baseline: nothing forced, nothing persisted, the daily schedule
  holds. Revert-proven (the old guard reddens the new pin with the exact
  pre-fix signature), pinned, ledger green.
- **/api/version** — the running app names itself. `GET /api/version`
  answers `{"name":"fleet","version":"<the compiled literal>"}`, pinned
  against the build's own `CARGO_PKG_VERSION` so a drift reds the suite.

## v0.5.0 — 2026-11-08

The resource-responsibility release. The increment is a real one in this
repo's own terms: the harness had been keeping everything it ever wrote, and
a harness that consumes all available disk is not a useful tool. Two
resource defects were measured, root-caused and closed, and the compositor's
paste path landed beside them.

### Fixed — what the harness kept

- **KI-478 — the checkpoint retention never ran.** `checkpoints::prune` was
  bounded and tested from the day it was written; its *trigger* was a manual
  verb and a session's `GoalDone` event, so a face up for days without a goal
  set reached neither. `~/.fleet/checkpoints` stood at **68 GB** across 72
  sessions — two holding 54 GB between them, one at 512 turn directories
  against a documented keep of 20 — and the face's log held **zero** hygiene
  lines for its whole life. The retention now runs on a 15-minute timer for
  the life of the process that creates the debris (`gui::serve` and the
  chat), through a live-safe pass that never deletes a capture still writing
  and never takes the turn a rewind points at. A turn's clock orders it,
  never its id: a session's ids do not restart with the process, so the
  owner's store held turns 13..16 written an hour after turns 493..512 —
  ordering by id would have spared the oldest turn and deleted the live one.
  The store cap also counts every byte the store keeps: summing only the
  spendable turns let a 5.2 GiB store report itself as under a 5 GiB cap,
  so the verb's own sentence was false and the pass stopped early. It now
  measures live — 5.07 GiB before a pass, 4.95 GiB when it returns. The
  daily maintenance script gained the leg for a machine whose face is
  not running.
- **KI-479 — the cache sweep had never once done its last tier.** It ran
  `cargo clean` under launchd's `PATH`, where `rustc` does not resolve, so
  cargo exited 101 and cleaned nothing while the script reported "the dev
  profile was cleaned whole". The exit code is now read, with an `rm -rf`
  fallback that needs nothing from a toolchain.
- **KI-477 — orphaned `mock-server` children** now die with the parent that
  held their stdin pipe, and the `target/debug/incremental` beside them was
  cleared (21 GB).

### Added

- The composer's clipboard paste, with the request caps raised to match what
  a real paste carries, and the send key's contrast fixed.

### Measured

Disk reclaimed on the owner's machine, each step verified: checkpoints
68 GB → **5.2 GB**, `target` 40 GB → **11 GB**, the volume's free space
**12 GiB → 116 GiB**. Memory was measured and left alone — every fleet
process together held 230 MB, with no leak to chase.

### Verification

The full gate ran green after the changes: **2,249 tests passed, 0 failed,
0 orphaned children**, `fmt --check` and `clippy --all-targets -D warnings`
clean, the ledger at 479 rows.

## v0.4.0 — 2026-11-06

The first GitHub release. The `0.3.0-beta` literal had been parked since the
repo's first weeks; the work below closed that out, so the release takes the
next stable minor number. Binaries: the native shell as a dmg and the CLI as
a tarball, both built from the tagged tree after the gate ran on it.

### Fixed — the open ledger, closed by class

The known-issues ledger went from **28 open rows to 5** at the closeout's
end (465 rows total); every row was re-verified against the code before it
was worked on, and one (KI-326) proved already fixed. The evening's own
scheduled corpus-refresh then found the KI-394 refusal shape a second time
and **KI-466** was filed, open — so this release's tree carries 6 open
rows, the new row naming its own fix-side choice rather than hiding the
regression.

- **KI-463** — a related-memories request built the whole embedding corpus
  synchronously on the request path (cold-corpus reads blocked the server);
  fixed so the request path reads the built caches and only the warm path
  builds, pinned by a source-shape test.
- **KI-464** — the telegram wait test waited on half its own assertion; the
  wait now watches both observables it asserts on.
- **KI-465** — a gate procedure that kept the summary and destroyed the
  failing names (`tail | grep`); the rule is now `> /tmp/<leg>.log 2>&1`
  with the exit read on the file — the release script's own gate runs this
  way.
- The nine classes the plan landed in one sweep: the nomenclature write
  guard, load-sensitive fixtures, the supersede/remember file routes,
  `reload` + delivery steps, child reports, account-state 400 failover, and
  plan hooks.

### Added

- **Releases.** `scripts/release.sh` gates HEAD on a detached worktree
  before it tags and cuts; `scripts/bump-version.sh` moves every version
  site in one step; this changelog is the release note, in one place.
- The first dmg deliverable, cut from the gated tree.

### Verification

The gate ran green on the committed release tree: **1,929 lib + 168
integration tests + 962 face-harness checks, 0 build warnings**, the ledger
and plan checker at exit 0.

### Install

See [the releases page](https://github.com/adelvillar1/fleet/releases/latest)
for the dmg and the CLI tarball, or
[build from the repo](https://github.com/adelvillar1/fleet#install).
