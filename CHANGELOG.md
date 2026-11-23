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

## v0.9.3 — 2026-11-23

**The surface pass, finished — and the ledger emptied.** Twenty-eight commits
past v0.9.2: every surface rebuilt on one grammar, routing folded into Reviews,
the known-issues ledger driven to **zero open**, the face harness moved to
Node 26 (the V8 segfault's actual fix), and a new instrument built end to end
from an approved mockup.

- **One surface grammar (the surface pass, R1–R10)** — the read-only spine
  strip retires from six surfaces (`d976eba9`); a stat is a door onto the
  section that holds its population (`c4199e74`); empty and loading states name
  their one press, and **empty is not refused** at the bridge (`37d8009b`);
  one page head (`answerHead`) with the design system's tokens holding
  (`a905d8ed`); the router's chains flatten and the address tells the truth
  (`0e19ee8d`); every div-button carries a cursor and answers the keyboard
  centrally (`393390cd`); a form inside a card is a section, not a second card
  (`217d6e59`); the card grid's spans are declared and measured with rails open
  and collapsed and the PWA at 390/820/844/1180 (`7df6d062`); a past run opens
  and the drawer's third tab is `work` (`1b40ed0b`). The method and the
  per-surface audit: `docs/architecture/2026-11-23-surface-audit.md`.
- **Connections is a knowledge page** (`8c131cac`) — the graph left the aside,
  its nodes drill in place, its legend chips isolate a kind, and a skill's row
  opens its lens instead of dying.
- **Routing is Reviews' first section** (`82f2f691`; mockup `4108b60c`) — the
  standalone Router page retires; `#/router` folds silently onto
  `#/calibration/routing`, which carries the live decision, the declared chain,
  cost × quality, the workload mix and the day's route rows; the strip follows
  the selected section.
- **The known-issues burndown** (`7710a81c`, `93c260ef`) — KI-560 (the provider
  card saved the *filtered* catalog, deleting disabled models; the read now
  publishes the declared `catalog_models`), KI-562 (the reload-all sequence
  drives `writeSettings`, holds its press, counts per provider, keeps its
  sentence across the repaint), KI-566, KI-584, KI-592 (the landing under the
  open drawer) and KI-594 closed with pins; **KI-595** found by the gate itself
  (a budget env leaked between parallel tests) and fixed the same hour. The
  ledger now stands at 593 rows, **0 open**.
- **The harness runs on Node 26** (`1ba31304`, `4a37a9b0`) — KI-593's V8
  mark-compact segfault is fixed by refusing older runtimes: both harnesses
  exit 2 below 26 and name the requirement, `tests/common::node()` resolves the
  runtime regardless of PATH order, the `--no-concurrent-marking` workaround is
  gone (the gate exercises the default GC), and 20 unflagged full runs on
  26.11.1 are clean. `FACE_PROGRESS=1` keeps a silent segv attributable.
- **Tokens by project** (`953a5213`; mockup `6883a54e`) — a new instrument
  plugin (`plugins/tokens`, untrusted, asks only `cache`): one card per project,
  the donut sliced by the models that worked across **every session scoped to
  that project**, the legend below it carrying each model's tokens and share,
  the provider's full breakdown on hover or focus ("not reported" as words), and
  three named buckets for the calls no session can claim. The route row now
  carries `session`; the cache document carries `projects` from the same walk,
  its own keys untouched.

Gate: fmt, clippy and the release build clean; `cargo test` **2,824 passed / 0
failed**; the face harness 2,007 checks across 290 cases on Node 26.11.1.

## v0.9.2 — 2026-11-23

**The map and the drawer — the rail becomes a map of work-kind regions, and
the drawer opens the object as a human would see it.** Forty-one commits past
v0.9.1: the face re-derived into the Indigo Studio palette and the drawer's
four canonical tabs first, then two plans built end to end on top of them
(`docs/plans/2026-11-22-ux-map-and-drawer-study.md`,
`docs/plans/2026-11-23-rail-map-and-drawer-bodies.md`, R1–R10, completed).

- **The rail is the map** (R1–R3, `cdf783e8`, `35b86631`) — **Home** (the
  inbox, with its held count) leads; `software · design · knowledge` are
  bracketed regions and the *system* strip closes the foot with the expert
  census and the enabled plugins; state is a dot or a count, never a group —
  the needs-you/working groups retire, and attention lives where the work is
  (the inbox by the age of the hold, the board's needs-you lead). Folding is
  per region, remembered; a quiet lane carries no dot.
- **The drawer opens the body the object is** (R4–R5, `613be4e7`) — one
  `KIND_MODE` table decides from the file read: code and trees → `code`,
  markdown, documents, office files and images → `preview`; the mode switch
  is the per-object override and never leaks into the next open. The read
  body opens with the relevance line (kind · size) and the one press it
  suggests — the sibling body, one press away.
- **Markdown renders properly** (R6, `79cc488e`) — marked 18.2.1 in the
  vendored set, output sanitised through DOMPurify (script and `onerror`
  stripped, proven live); relative images resolve through the contained
  preview route. The transcript's DOM-built renderer keeps its path presses.
- **xlsx shows the sheet's own form** (R7, `cea0986b`) — number formats,
  Excel serial dates, bold, merged spans, column widths, right-aligned
  numerics, `inlineStr` cells; formulas stay their cached values, labelled,
  and the 500-row cap says so.
- **pptx draws** (R8, `87edcde3`) — `@meshesha/pptxjs` 2.0.0 (MIT) renders
  slides inside the sandboxed frame from inlined bytes, no network; a viewer
  that refuses falls back to the text outline **by name**, carrying the
  renderer's own reason.
- **The map below 900px** (R9, `472c1d49`) — the rail drawer renders the same
  map (Home, regions, system), pinned at a 390 boot and captured at 390/820 in
  both themes from the running face (`docs/plans/mockups/2026-11-23-rail-map/`).
- **The pins read committed bytes** (`61335a07`, `31360dbd`) — a
  dependency-free generator writes the fixtures the pins use: a real workbook
  (merged bold title, serial dates, grouped numbers, percentages, widths, a
  501-row sheet) the harness inflates and renders, a docx boot, and a zip the
  deck renderer genuinely refuses. The URL kind has no drawer object — recorded
  as gap-ledger G3, not guessed into a dead mapping. Harness: 1,966 checks /
  285 cases; 2,820 Rust tests.
- **The face before the map**: the four themes re-derive into the **Indigo
  Studio palette** (`d4522fef`); the drawer reads in the **four canonical
  tabs** (`7e8d76aa`), the connections tab carries the goal edge and the
  narrow inspector retires (`588842c8`); the board leads with the home's
  needs-you decisions (`f5fee99c`); the age rides every inbox row that has a
  stamp (`205b3598`); the basis ladder rides the fact rows (`e709eb0e`); the
  learning graph is drawn (`fde071af`); health lanes are a live chip row
  (`ac273aec`); the fleet map renders the earned liveness as U16's meters
  (`ec9091dd`); the desk's stage track carries its names and the refuse-to-end
  gate (`6753e48c`); the reply carries the route that served it (`cef9565e`)
  and its evidence chips open the source (`9895c2de`); persona faces are
  vendored in-repo (`2937d0a5`); the bot PWA composes its split view and its
  discs are the persona face (`0e12aab2`, `20eda857`); the relay card reads
  the bridge's own counters (`43b8580f`); the version-page stand-in reads its
  head whole (`3364f452`).
- **The study behind it**: seven mock screens, 14 WebKit captures, 14/14
  in-page AA audits clean (`94594bf5`, `fe31f49f`), and the viewer research
  that chose the engines (`docs/architecture/2026-11-23-document-viewers.md`).
- One defect found by a live probe and filed, not fixed here: **KI-592** (the
  landing hero paints under an open drawer when `#/file` arrives with no
  conversation; pre-existing).

## v0.9.1 — 2026-11-22

**The relevance pass — surfaces lead with what needs the owner, then what
changed, and inventory folds below.** Nine commits past v0.9.0, built as one
contract (`docs/plans/2026-11-22-relevance-pass.md`) with seven red-proofs
that each name the item that vanishes when its derivation is removed:

- **One needs-you document** — doors owed an answer, approvals awaited,
  offers posted, ordered, each row one press away, consumed by every
  surface (`ab867bea`); plus the changed-since block: the six newest logged
  events across the owner's sessions, newest first, with a failed read
  landing as a named refusal, never an empty lie.
- **The landing leads**: needs-you, then what changed, then the archive
  collapsed and remembered (`aa9ca82e`).
- **The work list reads in the owner's order** — approval-needed, then
  running, then idle folded below (`2f545f7a`).
- **The door orders by the age of the hold**, and the count strip retires —
  a number without an action stops leading (`2157b508`).
- **The board leads with change and divergence; delegation leads with its
  blockers** — `never spawned` before `ran twice`, the tree beneath
  (`4e534cbc`).
- **The rail reads in states** — needs-you / working expanded, the rest in
  their categories as the idle inventory, filing by the strongest state a
  surface carries (`c0e4f206`; its red-proof ran close-out: dropping the
  needs filing dropped the inbox into its workspace category and the case
  said so by name).

Process made product-shaped in the same window: the **plan contract**
entered the house rules (a plan row on intake, never an appended thread —
the owner's ruling), and the **gap audit** reconciled eleven drafted plans
against what materialized (`6a8af9f2`, `fba8c608`, `f090467c`).

## v0.9.0 — 2026-11-22

**The milestone cut.** Fifty-three commits past v0.8.24 finish what that cut
promised and start three new lines of product, so the minor segment moves:
the face-rethought plan is **complete** — every row of every phase landed with
its own red-proof, verified against the whole table (this repo's numbering law
resumes third-segment discipline for ordinary user-facing changes after this
cut).

**The face plan closed.** P1's remainder (the receipts gutter, the composer's
nine controls decided and recorded in `UI-DECISIONS.md`, fork-at-turn, artifact
preview cards), P6's six system surfaces (settings' five postures over the
32-key mapping, the vault unlock map with its test-this-key probe, the plugins
manifest diff over a recorded ask ledger, the calibration scorecard with a
red-proved counterfactual replay, about folded, the armada deployment map),
P7's visual pass — the token tier, the display ladder, the card vocabulary,
the motion budget, the no-hex guard, the four sentences per surface, and the
hand walk measured at four breakpoints. Closing gate: **2814 units, 0 failed;
face 1839 checks across 271 cases**.

**The terminal got its intelligence panels.** The TUI's delegation panel joins
the face's graph document (per-node divergence words and the why-this-child
line), a **kanban panel** reads the work board the face reads, and a
**second-brain panel** carries the morning read — all through one law: the
same document the face reads, never a rederivation.

**The wire got 73 % smaller.** Every text asset the faces serve carries a
build-time gzip twin negotiated by `Accept-Encoding`: 4.0 MB → 1.08 MB across
19 assets; the shell path 2.05 MB → **555 KB**; a cold phone load at 1.5 Mbps
paints in **3.6 s** where it took 11.5 — measured, with the KI-587 clocks
re-verified as the regression floor.

**The phone got a third face.** The **mini console**: a 52 KB terminal-shaped
PWA under the relay with command-line navigation, reading the TUI panels' own
documents — cold through the paired listener: **436 ms painted, 19.7 KB on
the wire**. It carries the full KI-587 clock set (a stalled update answers
from the previous generation; nothing hangs), and two real defects were
caught while landing it: a boot clock that only lived in the script (a stalled
script left 47 s of silence) and a registration gated on `load`.

**The bot PWA became real.** Its worker registers versioned and precached,
the conversation view got the flex rule its composer needed (the dock sat
132 px below the fold on a phone), and the app lives under **`/bot/`** with
its own worker scope, so the console's worker can never claim it — measured
with both workers on one origin.

**The frozen PWA is fixed by three red-proved clocks.** The diagnosis
measured the freeze exactly: a stalled new asset plus a boot watchdog that
could not fire (it armed on `DOMContentLoaded`, which a pending module blocks)
plus a service worker that only handled rejection, never silence. Now every
shell fetch carries a deadline, the watchdog arms at script evaluation, and
an update claims its clients and sweeps one generation late.

**Fleet watches your multiplexer.** tuios (installed this day) now appears in
the armada map: sessions, windows by their agent words, Fleet's own pane
marked — by the **live command line only**; history and cwd are not evidence.

## v0.8.24 — 2026-11-21

Sixty-one commits inside the face-rethought plan: every clause of **P2 through
P5 landed with its own red-proof**, and the gate on the cut commit reads
**2739 units, 0 failed, face 1655 checks across 233 cases**. The plan itself is
**not complete and is not called complete**: its P1 remainder (the receipts
gutter, the composer's nine controls, fork-at-turn), **P6** (settings, vault,
plugins, calibration, about, armada) and **P7** (the visual pass — the appeal
mandate) remain open by design. Earlier cuts' notes calling this plan finished
were wrong and are corrected here.

**The work surface holds its contract, adversarially.** An inbox triage the
owner ruled into a deferral class shipped, was re-read by an adversarial review,
and that review found two hazards no passing test would ever name: the hold
store's read-modify-write was unlocked, so two concurrent accepts ran one
inbound twice, an accept and a decline both "succeeded" over one hold, and a
stale write-back could resurrect a hold that had already been accepted while
the webhook kept serving; and a corrupt hold store answered "held" for a
delivery that had actually vanished. The store now runs every mutation through
one lock, a second `take` of the same id fails, the unwritable store says **the
delivery was not held** instead of promising a hold exists, and the inbox
carries its read error rather than an empty queue. Five fix commits, each with
its red-proof in the message.

**Proof of life.** `/api/channels/rehearse` pushes a real `Inbound` through
`Driver::deliver` with no second device; the session's first record names
channel and peer; the feed derives `rehearsal {at, session, delivered,
answered}`; and the card says "the door heard you; no answer came back" when no
reply line exists — never green on faith.

**The delegation board finally joins the graph to the tree.** Delegation rows
carry their plan node; the graph document carries a per-node overlay (spawned
sessions, spawn count) with the divergence words **never spawned** and **ran
twice**, a graph-level count, and the why-this-child line drawn from the plan's
declared execution verdict and the child's judged shape. Rows written before
the field existed load unchanged.

**The skills surface earns its coverage map.** Fired versus unused, a
newest-first feed naming at/skill/session/surface, and the honesty rule: when
nothing records fires the block prints **"declared only — nothing records
fires"** instead of a zero dressed up as data.

**P5 closes.** The workspace-history scrubber asks the document for a moment
(`at=<iso|ms>` answers `as_of/covered`) and the four honest edges are pinned —
"before the first change" is never allowed to read as "no history". The graph's
weakest link is derived in the document and offered as the practice default
with the manual path intact. Version diffs compare **pixels** now — two panes,
a divider, honest "this version carries no image" instead of an empty frame.
And the design loop is a scored eval scenario (`scripts/design-eval.sh`): render
through the product's own capture, the defect read from the render's pixels,
the revision proved by a measured delta — the no-op red-proof caught the
report saying "the defect is fixed" over a 0.2% band, and the sentence now
reads its own measurement.

**Hygiene grew a reaper.** 1,232 orphaned `tail -f /dev/null` processes had
accumulated over ~55 days — 580 MB RSS, 46% of the per-uid process cap. `fleet
hygiene` now stops orphaned keep-alives **whoever spawned them** (an orphan has
no waiter, so stopping it cannot disrupt anything), guarded by an exact command
match and a dead-parent check, with `FLEET_KEEPALIVE_DRY_RUN=1` as a census.
Proven against real spawned processes.

**The screenshot rig is committed code, not a lost /tmp recipe.**
`scripts/gallery-seed.py` + `scripts/gallery-shoot.mjs` re-shot the gallery
through the product's own face; the run caught **KI-586** — the huddles stage
collapsing to 0px on phones (a one-rule grid fix; the desktop layout proved
byte-identical at Δ0.002/255) — and the rig refuses to commit an empty frame
instead of shipping a black rectangle. The delegation shot shows the new
overlay because the fixture had to learn delegation rows first.

## v0.8.23 — 2026-11-20

A gate that cannot be red at cut time, one environment lock for the whole
suite, and a build that went from twenty minutes to two.

**The test environment is written in one place.** 85 call sites set or removed
a process-wide variable across ~20 modules, guarded by three different locks —
`testsupport::env_lock`, a crate-level `TEST_ENV_LOCK`, and a
`layerfs_tools::EnvGuard` that took **no lock at all**, so 27 of that module's
tests mutated the environment unserialized. Two classes of flake had already
come out of it in one day (KI-578: a test inheriting the machine's git default
branch; KI-579: a `route` test reading a provider key out of the process). All
of it now goes through one `env_lock()` and one `EnvVar`, whose `Drop` restores
what was there — including restoring an *absence*, and including one guard a
test can move several times without leaking an intermediate value back.
`every_environment_write_is_a_named_site` scans the census and carries five
named production exemptions (the bootstrap, `adopt_login_path`, the `apply_env`
composer, the delegation's own trace variable, and the settings card applying a
choice to the running process); a new unnamed write reddens by file name. The
migration also found two real leaks, not races: `BLEND1_VECTORS_DIR` and five
`LAYERFS_*` variables were removed at the end of their tests rather than
restored, so a machine that had them set lost them for the rest of the run.

**Builds cost a third of what they did.** A cold release build measured 20m21s
twice. Five causes, four fixed: fat LTO → thin (warm relink 3m45s → 3m07s),
sccache wired through `~/.cargo/config.toml` (cold 2m14s, 454 of 1,000 compile
requests from cache), `jobs = 6` for the link's peak (2.21 GiB RSS on a 16 GiB
machine), and `[profile.dev] debug = 1, incremental = false`, which took
`target/debug` from **22.2 GiB to 3.9 GiB** and free disk from 39 GiB to 73 GiB.
A dev rebuild from an empty debug directory now measures 95s.

**Structure guards scan the crate, not a list.** The three beat guards read
hand-written arrays — 27, 30 and 42 modules — so a delegation guard saw 27 of
142 module files and 115 modules had never been looked at by any of them. The
census (`testcensus::MODULES`, asserted equal to the crate's own module tree in
both directions) replaces the lists: all three now scan 142, and a new module
cannot land without answering for it.

**Colour is not a cell.** `cards::fit` measured each word by summing
`UnicodeWidthChar`, so an ANSI SGR sequence counted as visible cells — `\x1b[90m`
charged seven — and a line that fitted wrapped anyway. `fit` is now
escape-aware: a sequence is carried, never charged, never split. The property
test had missed it because no fixture carried colour; one does now.

**The face tells the truth or keeps quiet.** One press contract
(`pressFeedback`, remembering its sentence per key so a repaint cannot wipe the
answer), one answer head (`answerHead`: eyebrow, title, sentence, verb — and no
counts in the head), one way to carry an object into the composer (`carry`), six
persona tints per theme with AA proved by arithmetic, and a topbar that shows no
pills and no zeroes on a fresh home instead of `no route yet · 0 · 0% · idle`.
The permission control is one cycling chip — ask → autonomous → full access →
ask — and it skips full access entirely when no working folder bounds it, so it
cannot arm a grant the server would refuse. The first-run door's ladder is
claimed per step (`done`/`now`/`next`), and a case now pins it rather than the
stylesheet's prose.

**Tests: 2,433 → 2,671** (+238). The face harness runs **1,497 checks across 209
cases**, up from 1,483 across 207 — every new case red-proved by breaking the
thing it describes first. Eight plans from the measured deep dive completed
(`gate-fmt-cleanup`, `run-checkpoint-reclaim`, `guard-list-inversion`,
`corpus-root-resolver`, `press-feedback-primitive`, `card-width-primitive`,
`build-cost`, `test-env-isolation`). Seven KIs closed (571, 575, 578, 579, 581,
582, 583); two noted (KI-577: single-shot runs never start the checkpoint
reclaim; KI-584: the harness cannot represent a `<select>`, so provider pinning
has no case and cannot get one on today's fixture).

## v0.8.22 — 2026-11-20

Delegation you can size, skills chosen by your brief instead of by file
modification time, and four bounds that can no longer drift in silence.

**Delegated child caps are configurable, and the cap is 100.**
`delegation.max_turns` had existed for weeks with a comment saying it
bounded children, and it bounded nothing: it read into the main loop's
`max_turns`, and the main loop is `capped: false`, so on the one path it
reached it did nothing while the delegation path — where a cap is real —
never read it. Meanwhile two hard-coded constants did all the work:
default 20, ceiling 50. Now `delegation.max_turns` (100) is what an
unasked child gets and `delegation.max_turns_ceiling` (100) is the most a
call may ask for; a call above the owner's ceiling is refused by name
quoting *their* number, and the subagent schema renders the resolved pair
instead of a literal. A ceiling set below the default is raised to it, so
the two never contradict each other. Depth still halves.

**Skills are ranked, not recency-cut.** The run's declared skill list was
`newest 60 by file mtime`, which put seven sibling video-composition
skills at ranks 55–62 on every run and told the model nothing about the
other 400. It is now ranked against the brief through the tool router's
own lexical ranker — pure, offline, no lane, no model call — and bounded
by a **token budget** (4,000) rather than a count, because the count said
nothing about what the entries cost. Measured on this machine's library:
460 skills declare ~107 at ~3,992 tokens, the same spend with the right
skills in it. What is held back is still loadable by name; the ranking
curates what the model is *told about*, never what it may load.

**A plan node's declared paths are a write boundary.** They were enforced
only as a dispatch lock — two siblings could not start at once on one
path — and nothing looked at what the running child actually wrote. A
child bound to a node now carries its `paths`, and a `write_file`, `edit`
or `apply_patch` outside them is refused before the approval gate, with
the refusal written as an instruction that names the file, the scope, and
the escape hatch. A node that declared nothing is held to nothing, and a
target the guard cannot resolve fails open with one warning.

**`fleet drift`.** The structure-guards doc published
`MAX_DEFINITION_TOKENS` as 10,050 for five crossings while the constant
said 10,800 — the document sessions read *before their first compile* was
wrong, and nothing failed. Ten rows now each read their real producer and
name the sites that state them (paths, never line numbers, because line
numbers drift); a text-scan test fails on exactly that disagreement, and
was red-proofed by reverting the doc.

**A tick is checkable.** A completed `todo` item now renders `verified`
or `unverified` against the run's own tool results — a command that
exited 0, a write that landed — bound by the harness rather than taken
from the model. An item with nothing behind it stays visible as a claim.

Smaller things in the same pass: the settings modal's height cap was a
flat 44 rows whatever the frame, so a 32nd knob clipped the provider
cards while sixteen rows of a 60-row terminal sat unused (KI-573); the
gate's fmt leg was red on seven files no session owned (KI-571); a clippy
lint and an env race in the armada tests kept the full suite from being
green on any machine (KI-572, KI-574); and `fleet run` sessions were
found to grow their checkpoint store with no reclaim timer (KI-575,
logged, not yet fixed).

One correction worth stating: this release's originating deep dive
claimed 404 skills were "unloadable by name". They were not — `use_skill`
always reached the whole library — and the dive is corrected in place
(KI-576). The real defect was discovery, which the ranking fixed.

## v0.8.21 — 2026-11-19

Armada becomes a fleet you can actually grow: containerized instances, roles,
a console, and a shared mind that all three of its classes travel over.

**An instance is a container.** `Dockerfile` builds a linux image — 122 MB,
unprivileged, one named volume at `/data`, the service lanes beside the binary
so they resolve in-image. `fleet armada join` is the container's entry: env
only, no prompts, every refusal named before any write. It clones the shared
corpus, mints the install-id once, registers on the flagship's roster
un-calibrated, and beats. A restart keeps its corpus and its identity
byte-identically. Two facts learned by building rather than reading: the
workspace's `default-members` pulls `src-tauri`'s GTK into a build an instance
cannot have, so the build is `-p fleet`; and `cpal` links `libasound`, needed at
compile time by the `-sys` crate and at run time by the binary.

**The shared mind carries all three classes** the armada direction note names.
`memories` through the corpus sync; **skills** cloned at boot (the canonical
library is its own repo — an instance is as smart as its siblings from the first
token, and says how many skills it loaded); **tools** merged from a shared
`mcp.json` with a deliberately asymmetric rule that is the security boundary —
a server definition arrives **disabled** (a shared file must not decide what a
machine runs), while an instance that already knows a server keeps its own
enablement and probe history.

**Roles as grant templates.** `fleet-owner`, `org-admin`, `operator`,
`auditor` expand into the existing grant vocabulary, so the guard needs no new
privileged path. `refuse_owner_tampering` refuses the owner principal and a
second `fleet-owner` in code, before reading the store — R3's actor-blind
boundary with teeth. `identity` resolves a run's principal from device context,
with owner as the unchanged default.

**A served control endpoint.** `fleet armada mcp` exposes the ten control verbs
as MCP tools over stdio, every call through the same guard as a relayed frame,
so the endpoint cannot become a second permission system. Destructive and
governance verbs refuse by name for lacking the fresh proof an agent can never
supply.

**Deploy, on OpenShip, over MCP.** The ssh command-drop that printed strings
nobody ran is retired. Deploy builds a `ServiceSpec` (so a dry run and a real
run cannot disagree) and provisions through a provider. The OpenShip leg speaks
its MCP endpoint — verified live against a real account: `tools/list` returns
432 tools, and the three calls the leg makes are real (`post_projects_by_id_services`,
`patch_projects_by_id_services_by_serviceId`, `post_deployments`). Docker remains
a provider for a local or remote engine.

**The console.** The armada lens carries the roster with heartbeat age rather
than a live boolean, the corpus's sync state, the guard audit stream, the deploy
card and a roles panel — a lens on the existing rail, not a second app.

**The expert switch is the census.** It listed nine lenses; now it lists every
surface the build carries, each with its subsystem name, and the palette speaks
the same vocabulary.

**A face run recalls from its own corpus.** Found while verifying: the face's
corpus resolution read the process environment's home, so a run under a scratch
home adopted the machine's real corpus. It now resolves through the run's own
home on both legs.

## v0.8.20 — 2026-11-19

The density button is gone, and a face run recalls from its own corpus.

**Dropping the density control.** Asked what it does, measured: its entire
reach was two CSS rules — a token trio consumed by three selectors (the deck
node's padding, a KPI tile's number size, the board columns' gap) and one
dashboard page's frame padding. The conversation, whose footer the button sat
in, has no compact mode: the 2026-10-31 density plan shipped single-line tool
cards unconditionally. Toggled in a real engine, the only moving number was a
KPI tile's height on pages the owner is not reading when they press it. A
control that changes nothing where the owner is looking is not a control. The
console's other controls (connections, labels, motion, expert, armada) are
untouched and were measured working both ways.

**A face run's corpus is its own home.** Found while verifying: the
folder-corpus integration test failed with every face change stashed — a
latent environment-dependent defect. The face's run path resolved its corpus
through the CLI's `resolve_root`, which consults `FLEET_HOME`/`HOME` first,
so a face under a scratch home adopted the environment's real corpus whenever
that held facts. The face now resolves through the run's own home on both the
folder-granted and the no-folder leg. Same shape as KI-087's rule, pointed the
other way: the home-first rule belongs to the headless process, whose
environment is its identity — a face's home is a parameter, not a fact about
the machine.

## v0.8.19 — 2026-11-18

The band at the top of the chat area is gone.

A strip above the transcript was showing text from a surface the owner was not
on — open the huddles page and its copy stayed over the conversations list.
The cause was one line: the room head's painter un-hid the huddles invitation
whenever the loaded session was not a room, on every paint and on every
surface. Since the state read runs on every event, any conversation could put
"Start a huddle. A working session you share with one or more bots." at the top
of its column, which is why it looked intermittent.

Each element at the top of the column now has exactly one owner, and the owner
guards its own surface: the invitation is drawn only by the painter that owns
it and only on huddles, and the room head only stands down there. Arriving at a
surface also clears the previous one's head, so no lens has to remember to
clean up after the one before it. Verified in the real engine across six
surface changes, and pinned by a case that walks the exact sequence from the
report — a conversation, then huddles, then back, then a repaint — which goes
red on the old line three ways.

## v0.8.18 — 2026-11-18

The collapsed rail's icons line up on the left.

The alignment fix before this one named the wrong edge, and it was mine: the
ruling quoted the owner asking for the badge-carrying rows to line up and the
icons to share an edge, and the rule I wrote packed every row to the RIGHT
(`flex-end`). The icons are what the owner scans, so the shared edge is the
LEFT one; the counts ride the far end of their own row, where a number
belongs. Measured in the real engine at the collapsed width: every row's icon
now starts at x=14 with a badge slot present, where `flex-end` put all of them
at x=57.

The face check that pinned the wrong side has been rewritten rather than
deleted — it now asserts `flex-start`, and separately asserts that the badge is
pushed to the row's far end with `margin-left: auto`, which is the half that
keeps the icons on one edge when a count arrives. Reverting the CSS turns both
red.

## v0.8.17 — 2026-11-18

The search reads the logs, and the queue drags.

**A typed query now asks the server.** The work list's box filtered the
titles, projects and models the page already held, so a conversation whose
BODY said the words could not be found by typing them. The server has
implemented `q=` since D9 — literal, bounded to the newest 200 logs, a
`matched` line per hit — and the face rendered that line while never sending a
query, which is why the two halves looked wired and did nothing. Measured live
against a real home in obscura: `q=warmup` returned 177 rows with 26 matched
and showed every one of them, and typing in the box sent no request at all.
Now typing sends the words, a row survives on the server's answer or its own
metadata, and the counts answer the search instead of the archive — 27 rows
become 2, with `2 matches for "warmup"` in the head, `earlier 2` on the group
and `active 2` on the filter. A match line counts only for the query it
answered, a miss names the 200 logs it read, and the head says "looking
for ..." while a newer ask is still on the wire.

**A queued message dragged to a new place lands there.** The row's label was
selectable while the whole row is `draggable`, so pressing on the text began a
native text-drag carrying the row's words; the drop handler parsed that as an
index, read `NaN`, and returned — the row followed the cursor and never
landed. The label is part of the handle now. Beside it, the desktop window
opened with Tauri's own drop handler at its default on, which claims an
in-page drag for the webview before the page sees it; the window now calls
`disable_drag_drop_handler()`, which costs nothing the face uses, since the
only `drop` and `dragover` listeners in the app belong to the queue row.

## v0.8.16 — 2026-11-18

The browser works, the board shows your work, and the harness writes to you.

**The browse lane runs on obscura.** It was a hand-rolled CDP client whose
`session_id` was declared, reset and never assigned: every page-domain call
went out sessionless and both engines answered `No page for session`, so the
only thing that ever worked was a no-op. The repair was unreachable — obscura
has no attach path, so the code that would have fixed Chrome could not be
validated against the engine you actually want. The lane stopped having a CDP
implementation: it is now a stdio proxy to `obscura mcp`, publishing **39
tools** — its three historical names aliased onto obscura's, the other 36 by
their own names. `browser_read` maps to `browser_snapshot`, and
`browser_screenshot` is translated, because obscura answers image content and
takes no path while the lane answers a path its callers hand to a vision model.

**Chrome is the last resort, and only on your word.** `browser.engine` is a
setting, `obscura` when unset. A host without obscura gets a failing check
carrying an **install obscura** press — from its GitHub release, both binaries
— with one line naming what Chrome would cost. The release is checked against
the published tag (current / behind / ahead, and *unknown* that never reads as
current), one bounded request cached for a day.

**A plan node finishes when its work does.** The completion branch returned
`None` and did nothing, so every node stayed `Running` forever: the board showed
the first task active and nothing after it ever moved. A completed child now
settles its node strictly — the declared gates run against the artifact — and a
node that does not verify becomes a named decision rather than a silent state,
because a node left running blocks every dependent and marking it failed blocks
them just the same.

**The board is your work at execution level.** One card per approved plan, each
atomic delegatable task in its queued / in-process / done column with the persona
it is assigned to, and a press that opens the drawer with its whole context and
its dependencies by state. Only an approved plan generates a graph, so a cron
run, a delegated child, an eval run and a probe cannot appear on it.

**The pane is contextual, and it is the drawer.** `files` reads the
conversation's own project folder instead of the directory the binary was
launched from, `work` shows this session's delegation as plain rows, and the
drawer became a pane tab *and its main component* — opening an object no longer
blanks the pane it just emptied.

**Sessions are named by their work.** The ladder is your rename, the standing
goal, the **first deliverable asked for**, the **first plan executed**, the first
message about the work, then the raw first message. The plan rung was reversed:
a later plan is the middle of the work, never its name. And 42% of this archive's
sessions opened with a harness instruction, so a subject rung now falls past a
smoke test to what the conversation went on to be about.

**Empty conversations belong to nobody.** A New press that never sent anything,
and a log whose every message names the machinery, are their own classes now —
derived from the log, not hand-marked — kept off your list and still countable.

**Core tools never ask.** Only the destructive class interrupts: `rm -rf`, a
forced overwrite, `write_file`, move and delete. Reading, searching, running the
suite and looking at an image run without a prompt, in `ask` as in
`yes-guarded`. The face also honours the `approvals` setting now — it had
hard-coded `ask` and ignored it.

**Paste one or more images with ⌘V.** The handler refused images outright. It
now attaches every image the paste carries, and the button attaches a list too.
A Finder copy no longer pastes the filename instead of the picture: images take
precedence over text, and a file's type is sniffed from its bytes.

**Vision answers.** It was the crate's only blocking routed caller, and the
blocking path never consulted the router — an empty base URL, a host-less
request, `RelativeUrlWithoutBase`. It rides the routed stream now, which also
gives it the walk's failover.

**Errors can be dismissed, and fixed.** Every message carries a dismiss press,
and a failing check that a press can fix carries it — the action mapped
server-side to a real endpoint, never guessed by the surface.

**The copy is written for you, not for the codebase.** The compiled prompt
carries the rule: say "service" not "lane", "the app" not "the face",
"approval" not "seat". The pages follow it — "the X read has not landed" is
"still loading", and a huddle says what a huddle is.

## v0.8.15 — 2026-11-17

Reviews and health stop wearing the same mark.

**Two rail rows, one drawing.** `ICONS` declared `'second-brain'` twice: the
project atlas grid near the top of the table, and a pulse added on
2026-11-15 when Health became a lens. A duplicate key in an object literal
silently keeps the last one, so the pulse shadowed the grid, the health row
wore the pulse — and `calibration`, whose two-beat signal is specified in
`docs/plans/2026-11-04-icons.md`, wore the same shape beside it. Nothing was
wrong with either drawing; what was wrong was that one of them could never
be reached.

**Health has a mark of its own.** A kit with a cross: a wide 15x12 body
against `gate`'s tall doorway, the cross at the centre. Measured at 17px
against every neighbour it could be confused with — the shield-with-a-cross
candidate was rejected for colliding with the existing `screen` shield, and a
boxed heartbeat for turning to mush inside 17 pixels. The address keeps its
key and gains a map (`ADDRESS_MARKS`): `second-brain` is both the Health lens
and the Intelligence section that holds the atlas, so the lens asks for
`health` while the section still names `second-brain`. The rail and the
palette read that one map, so they cannot disagree.

**The pin that should have caught it, and didn't.** The icons plan's guard
compares each rendered row's shape attributes as strings, and two pulses
drawn one unit apart are two different strings — so it was satisfied while
the eye saw one shape twice. Three checks replace that gap: no key may be
declared twice in the table; the reviews and health drawings must not
overlap; and no two marks of equal weight may rasterize to one silhouette.
The overlap is measured by rasterizing each path onto a 12x12 grid — the two
pulses scored 0.87-0.92 against each other where unrelated rail marks sit at
0.09-0.27.

Getting that pin honest took three attempts, and the failures are the
lesson. The first compared command skeletons with the numbers erased and
passed with the bug fully restored, because two pulses are genuinely
different command sequences. The second scraped the table with a regex over
the source, and a key declared on one line (`calibration: [[…]],`) has no
closing bracket of its own, so its body swallowed every following key. The
third read `shape.attrs.d` from strings the evaluator now returns, so every
path plotted nothing and every overlap read 0.00 — an instrument reading
nothing reports perfect separation. All three are red-proofed now against
both defect shapes, the duplicate and the two-pulse pair.

## v0.8.14 — 2026-11-17

The idle sweep stops disabling the tools it was measuring.

**No core tool is disabled for being unused.** The lifecycle sweep protects
a tool by name, and the list of protected names held ten — `bash`,
`read_file`, `write_file`, `edit`, `grep`, `glob`, `todo`, `remember`,
`supersede`, `subagent`. Everything else the harness declares was fair game
for a gate that disables what has not been called yet, so the sweep had
taken nineteen of the harness's own tools: `vision`, `deck`, `browse`, `ui`,
`web`, `plan`, `process`, `extract`, `use_skill`, `list_dir`, `apply_patch`,
`repo_map`, `file_change`, `convention`, `improve`, `frame`, `review_frame`,
`hybrid_search`, `mcp_resource`, `mcp_prompt` — plus five `blend1` lane
tools. `vision` went on 2026-10-31 with the reason `idle — 0 success in 0
call over 50 declared runs` and stayed off for seventeen days; the owner's
ruling is that this is the wrong reading. A tool nobody has called yet is
not a tool nobody wants, and the age of its last use is not evidence either
way.

The protection is now **derived** rather than remembered: every name
`tools::known_tool_names()` returns, the three capability tools, and every
`mcp__` lane tool are protected, so a tool this build declares tomorrow is
protected the day it is named. A lane's tools were already governed by the
lane's own lifecycle — a lane that dies withdraws its tools at the death —
and a second gate scoring idleness could only produce the same mistake. The
gate reads `state_of` and both manual guards (the CLI's `tools disable` and
the face's card), so `--force` is the one way past it.

`known_tool_names()` was itself the hole it promised not to be: the
function's own comment says "every tool name the harness can declare" while
omitting `goal`, `ui`, `extract`, `author_persona` and
`message_orchestrator`. Deriving from it would have inherited the gap, so
the five are named.

Pins: `no_core_tool_is_ever_swept_for_being_unused` walks every declared
tool at a hundred runs of zero use and fails if any of them reaches
`Candidate`; `the_protected_set_is_derived_from_the_harness` fails if a
declared or capability name is unprotected. The gate fixtures that used
`deck`, `vision` and `recall` as their idle subjects were repointed at
`fixture__*` names, because a protected tool can no longer demonstrate a
sweep at all. `fleet tools sweep` now reports `0 disabled, 42 skipped`, and
this home's `tools.disabled` is empty.

**Still open, and named rather than implied:** the `vision` tool is declared
and executes but cannot answer — `Client::routed` leaves `base_url` empty
and `api::chat` builds `format!("{}/chat/completions", base_url)`, so every
call fails `Bad URL: RelativeUrlWithoutBase`. And `reviews` and `health`
still share one rail mark: `ICONS` declares `'second-brain'` twice, so the
second declaration (a pulse) shadows the first (the project atlas grid), and
the pulse is the shape `calibration` already owns.

## v0.8.13 — 2026-11-17

The rest of the dogfooding day: the rails, the marks, and the update door.

**About is back on the rail, and a waiting release says so.** About had been
folded into Settings' advanced section, so the only door to the update apply
sat three presses into a place the rail never named. The Settings update card
compounded it: it announced that a new version was available, offered one
button that only re-checked the feed, and its own comment claimed a link to
About the code never built. About is a rail destination again, its entry
badges `new` (with the version and the way to take it) from the first paint,
and the update card carries the door its comment promised.

**Every surface wears the second rail, each on its own axis.** Health, inbox
and board were the only split pages without one, so each read as a single
scroll rather than a place. Their scopes are what their own data supports:
health by its checks (lanes, doctor, build — the machine's, not anyone's
project), the inbox by the five classes it already computed plus waiting and
archived (its chip strip is retired; the rail is the tab and the filter), and
the board by project, with an `all tasks` row kept alongside so nothing is
reachable only through a filter.

**The second brain is a category of intelligence.** The corpus and the
project atlas had been stacked on health beside the lane probe, sorting "is
the machine well" together with "what does it know". Both are Intelligence
sections now, with their reads and repaint guards following them.

**One meaning per mark.** The 2026-11-04 icons discipline had degraded in
ways its own pin could not see, because that pin asked "is every key the face
names drawn?" — and the fallback `dot` is drawn. So three Intelligence rows
had fallen through to one hollow bullet, three calibration ops shared a bare
check, and two marks simply did not read as what they were (`tools` was a
next-arrow; it is a wrench now). The map is complete, `route_judged` and
`validate` got the drawings the plan specified, and the pins now ask the
discipline's real question: every rendered row gets its own mark, and no two
share one.

Also fixed in the harness, so these judgements can be trusted: the
owner-inbox mock answers a re-read with the staged document, `setTimeout`
callbacks defer like a browser's instead of firing inline, `window.open` and
clipboard writes are recorded as effects a press can have, and four
second-brain endpoints are mapped instead of silently answering `{}`.

Still open, and named rather than implied: KI-566 (a settings read missing a
key can still throw the rail into a half-built state), KI-560 (re-enabling a
model on the provider card deletes it from the provider) and KI-562 (the
"reload them all" card still answers into the old silence).

## v0.8.12 — 2026-11-17

A day of dogfooding, and the buttons were lying.

**The press now answers where you are looking.** Both halves of the same
defect, and neither button was broken: `run the cycle now` on Intelligence
really ran the consolidation pass, and the second-brain `refresh` under
health really posted — and both said so in `#log`, the chat transcript,
which lives inside `panel-chat` and is hidden on every other lens. The work
happened; the confirmation went to a panel nobody had on screen, which is
indistinguishable from a dead button. `notice` now routes on whether the
transcript is actually visible and otherwise lands on a strip at the foot of
the stage, outside every panel, which clears itself after eight seconds.
172 of the face's 227 notice sites were on non-chat paths.

**The provider card edits its endpoint, not a caption of it.** The card
seeded its endpoint field from the row's display line — `base · KEY_ENV ·
enabled` — and posted that field back as `base_url` on every save and on the
disable press. Disabling a provider therefore wrote its own caption into the
endpoint, and every request then built a URL no client can dial. The read
publishes the entry's own `base_url` now, the card seeds from it, and both
writers refuse an endpoint carrying whitespace — before the key is stored,
so a refusal cannot strand a secret for a provider that was never written.

**The drawer stops at the stage's edge.** It was fixed to the viewport's
right, so at 1440 it covered the whole connections column — U13's "the
persistent panes never hide behind it," broken by the column it names. One
`--pane-right` property now carries the offset that the pane, its width
math, its grip and its dim all read, so they cannot disagree. The grip's own
clamp subtracts it, so a drag cannot widen the pane back over the column.

**Every button gets pressed, by a test.** Two sweeps drive the DOM instead
of reading it: every visible button on all 20 lenses, and every button on a
surface that only exists because something needs you — the approval seat's
decisions in both of its hosts, the inbox's five row classes with their row
and menu verbs, the service-repair verbs, a frame's action bar. 373 lens
presses and 248 acting presses now run clean, and a press that does its work
in silence fails the suite. Two harness blind spots came out of it and are
fixed: a `type=submit` press never submitted its form (20 live buttons
looked dead), and `setTimeout` ran synchronously, so no transient state
could be observed at all.

**The updater signature names its version.** The signer was called without
`--app-version`, so the signature was not bound to the release it ships
with — fine today, and rejected by any updater configured with
`requireSignedVersion`. The version now rides inside the signed trusted
comment.

Also fixed: a settings write hands its caller the write's own reply (every
reply-reading caller had been silently taking its empty path since the
helper was written), and a provider card's last-secondary arrow is disabled
with its reason instead of looking live and returning.

## v0.8.11 — 2026-11-16

Speak into the composer, and the day's three fixes.

**Voice dictation.** hex joins as the local transcriber — Rust, MIT, and
binaries for macOS, Linux and Windows, which is what makes the upcoming
Tauri ports platform-agnostic. The fleet process owns the microphone and
hex owns the models: the composer's new **Dictate** button records on the
server (cpal, one code path everywhere), spawns `hex service --embedded`
(ready in ~30 ms; a warm clip transcribes in ~0.11 s), and the words land
at the caret, ready to edit. One press starts, the same press stops; a
pulsing red dot is the listening state. `fleet dictate` is the same seam
from the terminal, and `--file` transcribes a WAV without a microphone.
Nothing is written to disk, a paired phone can never reach this machine's
microphone (the routes are loopback only), and a missing model is refused
in hex's own words (`hex model install <id>`). The native shell carries
the microphone usage description and prompts under its own identity.

**The collapsed rail narrows.** Collapsing the rail used to hide the
labels and leave the column at full width — the connections-column rules
re-declare the shell grid later in the stylesheet and out-specified the
icons rule. One live width token (`--rail-actual`) now feeds every grid
rule: the base, the narrow query, the tissue-on column and the reveal
strip, so the rail is 66px with the connections column on or off.

**The bot door is in the composer.** The add-a-bot door hung off the
conversation head, so it was absent on the landing, where a conversation
starts. It now lives with the ask: on an open conversation it joins the
picked roles; from the landing the same press **starts** the conversation
with them (the huddle creation path, no goal yet). The conversation head
no longer carries it; a room's members chip still does.

**Record keeping.** KI-547 corrected with the measurement that z.ai's PaaS
endpoint does report cache reads — a live run cached 10,176 of 20,519
prompt tokens (49.6%) — so the earlier zeros were head churn and
single-request runs, not a missing report; opencode's zero remains no
data. KI-554 closed (the help names every dispatched verb); KI-556 filed
(the updater signature is not version-bound).

Named and not fixed: **KI-555** — the open drawer is still anchored at
the right edge and covers the connections column, where U13 says the
persistent panes never hide behind it.

## v0.8.10 — 2026-11-16

The shell stops showing you things you did not ask for, and the docs catch
up with the app.

**The drawer opens only when you press it.** Desk and learning forced the
right drawer open on arrival — the only two lenses that did — and the force
overwrote your close on the next visit. The drawer is the owner's to open
(U13, amended): arrivals leave it closed, the lens's context is painted
into the closed drawer, and one press lands on that context tab (the
deliverable's or lesson's properties). A press that beats the lens's read
keeps the ask until the data lands. Pinned by the face harness; the live
probe on a fresh binary shows the closed arrival and the context press.

**The rail carries no install pill.** The card at the rail's head
(`fleet / v0.8.9 · 162 runs`) is gone — the version lives on About and at
`/api/version`, the run count on the archive, and the head mark already
names the install. Removing it uncovered the spill the card had been
covering: the rail head was a fixed half-topbar box (29px) around a 40px
mark, so the mark's second line sat under the first group. The head now
sizes to its content; the collapsed rail is the glyph and the icons, with
nothing spilling over the stage.

**The help names every verb.** `fleet cleanup` existed and every cut ran
it, but `fleet help` never named it — nor eleven other verbs the parser
dispatched: desk, workspace, plan, demand, hygiene, vault, plugins,
lifecycle, armada, footprint, recover. Each carries a line and its flags,
pinned by a test that reds when a dispatched verb is missing (KI-554).

**The docs match the app.** The README was re-read against 0.8.9: six
lanes (the browser lane was missing), nine lenses, the cache instrument and
its no-data rule, the orchestrator and the add-a-bot door, decks and their
opt-in clustering, cleanup on cut/deploy, folds at 85%, and the onboard key
path. Both archify drawings were re-authored at the release commit — the
architecture (16 components, 18 connections, two regions, four cards) and
the turn as the sequence it is (8 participants, 13 messages) — with fresh
PNGs, and the functional spec gained the cache surface.

Named and not fixed: **KI-555** — the open drawer is anchored at the right
edge and covers the connections column while it is open, where U13 says the
persistent panes never hide behind it.

## v0.8.9 — 2026-11-16

Caching you can see, and the bits that were hiding in plain sight.

The router's prompt cache is now measured. A new **cache** instrument
(Settings → cache) shows, per model, the share of prompt tokens served
from the provider's cache, the cached-versus-fresh latency, and a
day-by-day trend. The reason the share read low was the request head: the
tool declarations were re-ordered per workload, which rewrote the bytes
every provider's prefix cache keys on. The declarations now keep one
order, and the head carries a fingerprint so a miss is attributable — on
direct DeepSeek a second run with an identical head read 5,248 of 12,833
prompt tokens from cache. A provider that reports no cache figures is
left alone: the older gateways cache internally and send nothing back, so
their zero is no data, never a reason to re-rank the chain. Conversations
also fold later: at 85% of a declared model window instead of three
fifths.

**Adding a bot to a conversation** finally has a door: an "add a bot"
press on the conversation head and a matching chip in a room's member
row. Both open one picker — this home's roles, one press to join, and
"create a new persona" for a gap the catalog does not cover. The first bot
added turns a plain conversation into a room.

The **orchestrator** is legible: a route row now records the chain that
actually walked beside the workload the router judged, each role's card
draws its **work graph** — the threads it owns, arranged by the delegation
edges among them — and a conversation names its orchestrator.

The **deck** can build from a long document: its paragraphs are clustered
by meaning into blocks, each opening with its conclusion, with a
presentation getting a node per cluster and an infographic or poster
taking them as its sections. It ships **opt-in** (`deck.clustering`),
because the clustering's measured agreement (0.732) has not cleared its
0.90 gate — the measurement, not a hope, decides.

Cleanup is part of shipping: a run the resource contract curtailed queues
the files it wrote, and `fleet cleanup` — run by a release cut and by a
deployment — deletes them inside their own folder only. A cut also sweeps
fleet's own `/tmp` debris past a day's grace.

Smaller: `/api/settings` answers in 18 ms warm instead of 1.9 s (the
lifecycle scan is cached and the board no longer walks every session log);
the known-issues ledger is at zero open, and its plan statuses speak one
word with a staleness check so nothing sits active for no reason.

## v0.8.8 — 2026-11-15

The app sees the machine it runs on, and the work you started is always
findable.

The desktop app now adopts your login shell's PATH at startup. A
dock-launched app inherits launchd's four directories, so every tool you
installed elsewhere was invisible to it: `docker info` could not be
spawned and the health card called a healthy kiri daemon dead, three MCP
servers failed with "No such file or directory", and the blend1 lane was
handed macOS's python 3.9.6. One read of your shell's own PATH fixes all
three — bounded, silent when it fails, and never asked twice. The card's
down-message now names the check that failed instead of asserting a
daemon's state.

Findability: the archive reads the owner's conversations and the
machine's scheduled runs with separate windows, so a store whose cron
jobs fire every few minutes can no longer crowd the work you started out
of the list, the archive toggle or the search — the cold landing carries
its way back too. And the delegation graph is legible: a conversation's
delegated children ride under the parent's goal, named, listed, and one
press from opening.

Smaller: the footprint budget learns a high percentile instead of the
median, so a release is not refused by its own average; the dmg step
ejects stale fleet disk images before bundling.

## v0.8.7 — 2026-11-15

Fleet heals more of itself — the self-healing review, built out.

The substrate was already self-healing; this release takes the rest of
the review's candidates. The API probe makes a real no-token request, so
a key that is set but refused (or an API that is unreachable) names itself
before a run waits. A wrong vault passphrase and a corrupt checkpoint
manifest are named, not swallowed. A full disk triggers fleet's own cache
sweep once. A cron job that fails on its schedule gets one bounded retry
on the next tick. The health loop now also names a lane that cannot answer
and a broken MCP server (surfaced, never auto-disabled), and a cron thread
that dies restarts itself. On the owner's word: a bad update's previous
build is restorable, the lane venvs rebuild, and the corpus syncs — each a
card that asks, then acts. Corpus drift shows on the health lens, and a
corrupt device store is named instead of painting zero devices.

## v0.8.6 — 2026-11-15

The substrate heals itself, and the desk is project-centric.

The kiri/Docker substrate is now self-healing: when the worker container
is stopped the app starts it itself and the card shows "starting
container…" — progress, not a press; a sleeping runtime is woken; and when
no container runtime is installed at all the card offers "Install kiri and
start the container", downloads the kiri installer and opens it, then
starts the worker once the runtime answers. The messages name the runtime
honestly now (kiri or Docker), retiring the stale "start Docker Desktop".
The manual Repair press stays for a worker that will not auto-start.

The desk landing is project-centric: deliverables group by project — the
briefs and the design boards beside each other — with a project selector,
and runs are the fallback only when nothing has been made. The expert
switch, which flipped a flag nothing read, now reveals the subsystem name
behind each plain lens label.

## v0.8.5 — 2026-11-15

The project-centered workspace — the capability the 0.8 rebuild buried is
back, reachable through the spine instead of hidden in Settings.

The desk reaches a project's design deliverables: a design board is a
deliverable now, joined to the same project as the desk's own work, so the
project column and the shelf carry it and a press opens the canvas — the
composer's promise of "its sessions, its design, its facts" kept. The
second brain is a visible **Health** lens at last: the rail names it, the
palette finds it by "health", and the lanes' live probe and the doctor's
checks render beside the atlas (the inbox "open doctor" press opens it
too). Routing's live decision and tier chains ride a Reviews section; the
connections column joins a delegated helper to the run that sent it. The
Advanced surfaces answer to their plain names — secrets, assistants,
versions — not only their subsystem labels. The rail stays lean: one lens
added, the rest woven into the lenses already there.

Also from the sinkhole pass: a checkpoint names when it was held and what
the cap left out, and a cron fire's drawer opens the session it ran in.

## v0.8.4 — 2026-11-14

Draining the sinkholes and closing the dead ends — two audits of the
places fleet produced information it never resurfaced, and the journeys
that left the owner stuck, answered in one pass.

The calibration lens now shows every measurement the stream records. The
seven ops that were written and never displayed — footprint, shape,
curate, validate, demand, learn probes, learn checks — each render their
own facts, grouped beside the three the lens already carried; the curate
vote was being dropped at the read gate and now survives it. The facts a
run already transported but no surface read come into view: which corpus
a recall came from, a checkpoint's size, the turn cap beside the climbing
counter, a cron fire's catch-up mark and its session, how many skills the
run carried, the repairs the grader applied, and a skill's last-modified
time and canonical library. The recall-over-past-sessions channel opens
itself on first use instead of answering "not searched" forever, and the
routed event carries one order shape, not a shadowed duplicate.

The dead ends close on the face: a refusal gives the owner their words
back — the goal that set but whose run was refused, a queued message the
server turned away, a plan that would not send — instead of destroying
them; the borrowed composer on the desk and a lesson queues while a run
is in flight like the main one; the seat's decision buttons travel with
the composer that raised them; a relayed face declares its connection
lost like a direct one; a stop pressed during a network gap replays when
the server is reachable again; and the archive, open, and rewind presses
name themselves when a gap swallows them rather than doing nothing.

## v0.8.3 — 2026-11-14

The orchestrator — the owner's hub topology, built on the routing and
persona machinery the 0.8 line already carried.

A persona can name its workload: the role file's `workload` key names one
of the routing table's own workloads, and every run dispatched under it
walks that chain — sys1 still judges every task and the decision row
records both labels (`walked_as`), so the standing choice is graded
against outcomes like any other route. A conversation carries an
orchestrator assignment: the lead persona's workload is what its runs
walk, and an unassigned conversation walks the leader's `hard` exactly as
before. The role owns the threads assigned to it end-to-end — a durable
attribution index (`sessions-roles.json`) is written at dispatch and
survives disposal, archiving, and the spine's budget; the persona's card
lists its threads and its own work graph beside them. The topology is
hub-and-spoke: a dispatched persona has `message_orchestrator`, a line to
the conversation that dispatched it, delivered attributed through the
control channel; same-level personas do not address each other directly —
a member's address to a peer in a room reroutes to the lead with an
*unresolved dependency* signal, the visible evidence that the task
breakdown missed a dependency. Roles are also git-tracked in the store
convention, and the pairing card mints in the app with no terminal
command anywhere in the flow.

## v0.8.2 — 2026-11-13

The substrate diagnosis tells the truth about which failure it sees. A
machine restart reassigns Docker's published port and the worker's
endpoint record cannot follow — the probe's refused path now asks Docker
for the running container's actual port and names the stale record with
both ports ("the worker answers on port 57888, not the 60207 the endpoint
record names — the record is stale"), instead of reporting KI-493's
dead-worker shape for a worker that is up. A record that agrees with a
dead container still reads KI-493. And the repair press repaints
honestly: the reply is the HealthReport document the face reads, so the
card flips from the press's own answer instead of corrupting the face's
state until the next health event. The repair itself needed no change —
it reconciles the record and the worker in one press.

## v0.8.1 — 2026-11-13

The dogfood pass — the owner ran fleet on fleet, and every defect the run
surfaced is fixed at the mechanism.

The drawer is the one right surface: the chat's receipts column folded in
as the pane's made-here tab, the phantom grid tracks retired, the stream
steady (instant stick-to-bottom, text nodes appended per frame, the run
card's tick in place), and a queued message sends on demand — send-now
stops the run in flight, the composer button queues while busy, Stop
lives on the working line. The warmup is situate-and-align: lean skills,
a compiled-prompt carve-out that ends with the ask, effort scaling and an
anti-second-guessing rule — a bounded probe now lands in 11 turns where
the audit ran 60. The warmup never executes what it finds open: an active
plan is context, not authorization. Pairing is in-app — the pair press
brings the remote face up itself, relay origins persist as settings, and
no terminal command appears anywhere in the flow. The substrate repair is
an in-app action: the down layerfs card asks "do you want to repair it?"
and the press runs install's own leg, repainting every face on the health
event. Four themes remain (paper and graphite retired, U48) and the
Motion switch's press always lands — the OS reduce setting is the default
until the owner takes it. The landing is the question (U49): one line
over the composer plus the onboard door, eleven explainer surfaces
retired, /api/home with them. The skills store is git-tracked with its
drift reconciled, and its pre-commit hook's warn-and-pass is real.

## v0.8.0 — 2026-11-13

The workspace rebuild — the face is one workspace, not twenty panels.

The three approved mockups became the shipping shell: seven lenses (threads,
board, desk, intelligence, learning, reviews, inbox — fleet when armada is on)
replacing twenty subsystem destinations; one spine read carrying every work
item with the edges it earned (a run links the facts it wrote and the skills
it drew); one shared focus that a pivot re-projects into the lens it names;
the inspector pane as a drawer, never a fourth column; the panel registry
retired — the panel set derives from the route table; the legacy skills
surface folded into Intelligence (the old `#/skills` address walks to the
segment); the 12px type floor and readable text tokens; plain language by
default with the expert toggle; `core/store.js`, `core/router.js` and
`core/dom.js` as the face's first native ES modules. The beta bar is
untouched — 0.x is still beta, the owner's call.

## v0.7.5 — 2026-11-12

The connections column collapses from its own door.

A refinement on the workspace shell: three doors on one state
(`body[data-tissue]`) — the console switch, a chevron in the column's own head
(visible while open), and a slim edge tab holding the stage's right edge while
the column is away (visible only while away, ≥1081px — below the breakpoint
the column is dropped and the tab with it). Collapsed is a 30px strip, not a
vanish: the way back never hides with the pane it controls, the choice
persists, and every door syncs off the same paint.

## v0.7.4 — 2026-11-12

The workspace shell (S6) — the pivot renders in place, and one read feeds the lenses.

**Part of the armada vision** — the owner's second steer on the reskin: *"but
this is not workspace centric, the mockups are."* The audit agreed and this one
was architectural: the pivot still navigated panels, and every panel fetched its
own store. Focus joined the silos; it did not remove them.

- **The workspace spine** — one read (`GET /api/workspace`) over the existing
  stores that the shell's lenses project from: tasks, deliverables, threads,
  instances, and the intelligence segment counts. The center is a lens over
  this context, not a panel that owns its own store.
- **The pivot renders in place** — the lens's address change rides
  `replaceState`, which fires no hashchange: the pivot never leaves the
  workspace through a panel-swap navigation, it re-projects the same context.
  The connections column and the focus law carry over unchanged.
- **The board projects from the spine** — the tasks lens no longer owns a
  separate fetch; the workspace context is its single source.

## v0.7.3 — 2026-11-12

The structural overhaul — the workspace becomes one graph, not a drawer of
separate boards. The owner's steer mid-flight: *"I'm worried that the project
you are executing is just a reskinning and not the complete overhaul of the UX
that was the project's original intent."* The audit agreed: the earlier cuts
laid the visual system and several panes, but the approved design's core — one
focus, lenses as views over the graph, dispatch streaming in the workspace —
was not wired. This release wires it. The plan is
`docs/plans/2026-11-12-ux-reskin.md` (the structural spine S1–S5).

- **The one focus (U3, S1)** — the shell holds ONE focused object; the
  connections column, the focus card, and every lens follow one pivot. Opening
  a deliverable / thread / lesson focuses that node; clicking a node opens its
  lens. No surface keeps a private second focus.
- **The dispatch hero flow (U20, S2)** — dispatching a task mints it, opens a
  thread for it, and the run streams THERE, in the workspace — not a modal. The
  graph grows the task→thread edge at dispatch and the task→deliverable edge
  when the work lands on the desk (the task settles against the end gate).
- **The console + the chrome search + the armada switch (U12/U24/U29, S3)** — a
  permanently visible control region whose every control applies live (density,
  connections, labels, motion); the chrome search filters the active lens (the
  palette is the other door); the armada switch grows the graph with the
  roster's instances and the shared-mind edges — off leaves fleet whole.
- **The board (U14/U21/U23, S4)** — the work as working/blocked/done columns
  with drag, the task cards with their bulk check, tags, link dots and running
  sweep; bulk is explicit; a status move is optimistic-then-settled. The inbox
  row wears the resolved principal chip (U17).
- **The TUI status color law (U15, S5)** — working amber, blocked red, done
  green, the same three the face wears; fleet-desktop wraps the reskinned face
  over the same server, no second frontend (U29).

Refinements named on the plan, not hidden: the task→lesson edge rides the
learning distil flow; a signal-accent picker is not yet a console control; the
omnichannel outbox states ride the communication layer (lane 2); the armada
provision gesture is CLI-driven today (`fleet armada deploy`).

The face harness carries it: **1192 checks across 164 cases**; the four-leg gate
green at every landing.

## v0.7.2 — 2026-11-12

The U-reskin's second, third and fourth cuts land together: Intelligence,
the thread pane, and the desk loop + the epistemology. The plan is
`docs/plans/2026-11-12-ux-reskin.md` (the phase numbering maps to commits —
8f24b84 + 7662315 + d0ec587 are Cut 2, 911c1ab Cut 3, 7472ef9 Cut 4 — and
this release carries the three, each verified and gated as it landed).

- **Intelligence (U40)** — the lens formerly named Memory, four segments on
  its own section rail: Memories (the corpus and the Dream, untouched),
  Skills (the library's own rows, the budget press, the pane card, the
  proposals with **apply through the gate** — the skills surface folded,
  `#/skills` walks to the segment), Tools (the capability registry: builtins
  with their gate-class chips, the vendored lanes, the MCP registry's
  servers with a real enable switch written through the settings writer's
  own server op), and Workflows (the declared graph's chains, the critical
  path, the honest undeclared empty). The reads: `/api/tools` and
  `/api/workflows`, over the existing registries.
- **The thread pane (U39)** — the receipts of the run beside the
  transcript: deliverables, facts recalled and written, tasks with their
  stage, the grades it landed — dotted by kind, fresh rows pulsing once, a
  click pivoting the workspace. It collapses to the conv-head's tissue
  chip and the fold persists; the composer is never covered; ≤1080px it
  stacks below the chat. The composer's pinning (U20) verified as already
  structural.
- **The desk renders the loop (U30)** — the deliverable card carries the
  five-stage track with the current stage lit and the named refusals as
  red chips ON the card; the shelf rides the desk landing again.
- **Calibration is the epistemology rendered (U31)** — the judged rows in
  their own card: workload, the verdict chip, a confidence meter whose
  width IS the number, and the owner's word riding the same append-only
  row (gold border when it does) through `/api/calibration/dispose` — the
  CLI's disposition verb surfaced, the word validated against the store's
  vocabulary.
- The face harness carries it all: **1163 checks across 155 cases**; the
  four-leg gate green at every landing.

Open on the plan: C2's tick awaits the owner's ruling between a per-builtin
disable (a new capability, outside this plan's reads-only scope) and reading
U40's switch as the registry's; C5 (onboarding) and C6 (TUI + desktop) follow.

## v0.7.1 — 2026-11-12

The U-reskin's first cut: the design system lands on the production face.
The plan is `docs/plans/2026-11-12-ux-reskin.md` (Cut 1 of six), implementing
the approved ledger `docs/plans/UI-DECISIONS.md` (U0–U41) — the owner's words:
*"we are looking for connective tissue and to eliminate the disjointed
surfaces."*

- **Harbor Night is the ground (U4/U5)** — the deep ink-teal default with the
  nine signal hues carrying fixed meanings, one-to-one with the approved
  mockup; **Day Watch** is the warm-paper inverse; paper survives as a theme
  of its own. The `themes.rs` contrast guard learns both and now measures
  each theme's own gradient label (`--dsw-alias-on-accent` — KI-476's law,
  generalized): night holds 5.40:1 worst, day 4.51:1.
- **The three type voices vendored (U6)** — Space Grotesk, Inter, JetBrains
  Mono as latin variable woff2 served from the binary (hard rule 12: the face
  fetches nothing), the display voice re-pointed to the grotesk; the U7
  geometry tokens land (`--r-lg/--r-md/--r-sm`, one deep-soft shadow).
- **The shell carries the tissue (U9/U10/U11)** — the rail reads Mission card
  (workspace, version, run count) / the surfaces groups / Saved views / the
  tissue explainer; the grid grows the permanent 320px tissue column at
  ≥1081px, rendering the work-object graph from the new `/api/tissue` read —
  tasks, threads, bots, deliverables, lessons, the edges between them, the
  typed legend, the focus card, a click pivoting the whole workspace.
- **Motion (U8)** — one ambient canvas behind the shell, the grain layer, a
  Motion switch in the chrome that freezes everything, and
  `prefers-reduced-motion` honored automatically.
- **The face harness carries the shell** — six RESKIN cases (night default,
  the motion freeze, the Mission card, saved views, the tissue paint and
  pivot, the honest empty note): 1141 checks across 147 cases, green; the
  four-leg gate green.

Cuts 2–6 follow: Intelligence (U40), Threads (U20/U39), Desk + Calibration
(U30/U31), Onboarding + setup (U34–U38/U41), and the TUI + fleet-desktop
color law.

## v0.7.0 — 2026-11-12

The armada feature set complete: fleet deploying fleet, and the surfaces.

**The 0.7.0 milestone** — this release completes the armada build program
(layers L1–L9). Every contract armada needs now works for one owner; lane 2
(the org) is a generalization — more principals, roles as grant templates,
per-user factors, team/org scopes — not a rewrite.

**Part of the armada vision** — layer L9 (deployment + surfaces), the body and
the face. The plan is `docs/plans/2026-11-11-armada-l9-deployment.md`.

- **Deployment, fleet deploying fleet** — provision a target with the
  self-contained binary: a local spawn lays out a home, mints a fresh
  install-id, and clones the corpus remote (inheriting the mind, not the
  identity); a VPS is reached over SSH and dropped the same binary from the
  release channel, credentials out-of-band. The fresh install registers with
  the flagship and is trusted only after doctor + settles — never by arrival.
- **The surface progression** — one `src/armada` library, four surfaces:
  `fleet armada {roster|status|sync|deploy|dispatch}`; a genuinely visual TUI
  armada board (liveness, capacity bars, reliability gauges); a face armada
  section (roster board, dispatch composer, corpus view) in the
  centre-conversation + context-pane shape; fleet-desktop wraps the same face
  over the same server/bridge — no second frontend, no second transport.

**The armada stack, delivered across 0.6.31–0.7.0:** L1 identity, L2 journal,
L3 security, L4 corpus sync, L5 communication, L6 relay broker, L7 workspace,
L8 control plane, L9 deployment + surfaces. Trust is earned by measurement at
every layer; authority rides signed in the frame and is verified before
obeying; the relay stays a dumb broker.

## v0.6.38 — 2026-11-12

The armada control plane: the hand that commands instances over the relay.

**Part of the armada vision** — layer L8 (control plane). The plan is
`docs/plans/2026-11-11-armada-l8-control-plane.md`.

- **Registration + roster + heartbeat** — an instance registers on join
  (install-id, sid, version, capacity); the flagship roster holds it. Liveness
  is earned by heartbeats, not assumed — a sid with no recent heartbeat is
  reported not-live.
- **The authority transport** — every control frame carries the principal and,
  for destructive/governance verbs, a fresh assertion signed by the flagship's
  Ed25519 key. The instance verifies the signature before obeying; a forged,
  unsigned, stale, or mismatched assertion is refused by name. The relay stays
  a dumb broker — it cannot fabricate, an instance cannot escalate, an agent
  cannot borrow.
- **Key custody** — the flagship mints the Ed25519 key once (PKCS#8 0600); an
  instance holds only the public key, distributed out-of-band. It can verify,
  never sign.
- **The control vocabulary** — dispatch/state/corpus-sync/version-roll/retire/
  health/register/heartbeat, each through the instance's own guard and
  admission after authority is verified; a privileged verb clears step-up (the
  human second factor). corpus-sync triggers the L4 sync.
- **Autonomy earned by measurement** — a fresh instance is un-calibrated until
  it settles runs; a reliability beat grades dispatch outcomes over the journal.
- **Relay-down is a named degradation** — instances run autonomously, the
  corpus still syncs over git, live control resumes on return.
- **The surface** — `fleet armada {roster|status|sync}`. The flagship role is
  additive: an install with no roster behaves as today.

## v0.6.37 — 2026-11-12

The armada workspace: the work-object graph that connects the four stores.

**Part of the armada vision** — layer L7 (workspace), the work-object graph.
The plan is `docs/plans/2026-11-11-armada-l7-workspace.md`.

- **The work-object graph** — `src/workspace.rs` keys work by object, not by
  folder: a `WorkObject {id, kind}` over {task, thread, deliverable, bot,
  lesson}. Only the task is store-owned; the rest are typed references derived
  live from the session/desk/roles stores (no parallel store). Tasks and edges
  persist as append-only JSONL folded on read, in a home exempt from journal
  retention.
- **The first-class task** — `Task {id, status, assignee}`: a stable
  cross-session id, a working/blocked/done lifecycle, and an assignee that is
  L1's principal. The `todo` renderer is untouched; this is the additive unit
  armada assigns.
- **The hand, fixed** — `hand_to_desk` binds the room's own workspace (not the
  server cwd), attaches the room's thread (`desk.session` was `None`), and
  records the thread→deliverable edge.
- **Surfaces as views** — `projects()` is a projection of the object-keyed
  `graph.view`: the same deliverable/lesson rows, now object-keyed at the
  source, plus tasks grouped by their workspace binding. The face is unchanged.
- **Bot invocation, gated and graded** — `task.assign` admits through the L3
  guard under the assignee principal, runs under the role overlay (a huddle
  stays a session), and carries the deputy principal. The task reads `done`
  only when the deliverable passes the desk end gate, otherwise `blocked`,
  named; every mutation settles a `work` journal event.
- **The surface** — `fleet workspace` prints the tasks, edges, and the
  object-keyed view.

## v0.6.36 — 2026-11-11

The armada omnichannel edge: communication that loses nothing.

**Part of the armada vision** — layer L5 (communication), the responsible
channel edge. The plan is `docs/plans/2026-11-11-armada-l5-communication.md`.

- **The durable outbox** — a reply is journaled before the attempt, retried
  on failure, and curtailed to a named `undeliverable` past its budget. No
  reply is silently lost.
- **The identity graph** — a peer on any channel resolves to one principal;
  an unlinked peer is a named refusal, never a silent owner fallback.
- **The bus is the journal** — the channel driver recovers seats and
  outcomes from its persisted cursor on a lag or restart; the broadcast's
  silent drop is retired.
- **Gated dispatch** — an inbound run acts for the peer's resolved principal,
  which the guard reads; every dispatch settles a journal event.
- **The telegram offset persists** — a restart resumes the long-poll rather
  than re-reading history.

Single-owner behavior is unchanged: paired peers act for the owner, an
Observe peer is still refused, the synchronous happy path stands.

## v0.6.35 — 2026-11-11

The armada shared mind: the corpus syncs across installs.

**Part of the armada vision** — layer L4 (the shared corpus), the distributed
knowledge. The plan is `docs/plans/2026-11-11-armada-l4-corpus.md`.

- **The sync** — `fleet memories --sync` is fetch → merge → push over git;
  fleet owns the semantic merge. With no remote it refuses by name and the
  corpus stays local. Opt-in and additive.
- **The merge rules** — recall-count merges by sum/delta (a fact recalled on
  one install bumps its rank for another); replacement serials are
  install-scoped so two installs never collide; a supersede collision
  resolves to one live frontier by the basis ladder.
- **The feeder** — `--pull --source armada` reads a remote install's corpus
  at its git commit through the unchanged import path, provenance-tagged.
- **A responsible citizen** — only the tracked fact tree moves (never the
  recall index, locks, or per-install state); every sync settles a journal
  event and is graded by read-back; an unresolvable conflict curtails, never
  a half-merge pushed.

## v0.6.34 — 2026-11-11

The armada nervous system: a self-contained relay broker.

**Part of the armada vision** — layer L6 (the relay transport), the control
lane's foundation. The plan is `docs/plans/2026-11-11-armada-l6-relay.md`.

- **The broker** — `fleet relay --broker` runs a dumb switchboard that routes
  sealed frames by session id, a mode of the fleet binary. It replaces
  `relay.py`: no Python, no foreign runtime. The fleet client connects
  unchanged; the sealing is end-to-end and untouched.
- **Push with ack + cursor** — a frame is trusted delivered only when acked;
  an unacked frame is a named gap the client re-requests, never a silent loss.
- **The control frame** — `kind:"control"` carries the principal and (for
  step-up) the assertion; the broker routes it dumbly, parsing no verb. The
  vocabulary is the control plane's (L8).
- **Liveness earned** — a session is live only while it polls; the queue is
  bounded and names a breach. The roster (L8) consumes this.
- **Surface** — `fleet relay --broker` runs it; `fleet relay` reports status.

## v0.6.33 — 2026-11-11

The armada authorization layer: a principal-agnostic security substrate.

**Part of the armada vision** — layer L3 (security), the owner's example of a
layer fitted into fleet that exposes well-defined contracts armada consumes.
The plan is `docs/plans/2026-11-11-armada-l3-security.md`.

- **The grant** — `(principal, dimension, scope, action-class)` with closed
  sets; the folder grant is the first row. Default-deny: an ungranted action
  is refused by name.
- **The uniform guard** — checks the grant at the one choke point every call
  crosses. Lane 1's owner holds a wildcard, so nothing the owner does today
  is refused; every decision is audited to the journal.
- **The declaration filter** — a principal sees only the tools it is granted;
  an ungranted tool is not even declared.
- **Step-up** — a destructive/governance action needs fresh human proof:
  lane 1's in-turn approval, or a signed Ed25519 assertion the agent cannot
  forge (the org-scale form, issued by the flagship at L8).
- **Governance** — the grant store and identity registry are immutable to the
  agent; grant-making is the owner's act, refused by name.
- **Surfaces** — `fleet grants` and the settings grants card.

Principal-agnostic from the first cut: the org multiplies principals and
leaves this code unchanged. Single-owner behavior is unchanged.

## v0.6.32 — 2026-11-11

The second armada bedrock: a durable, total-ordered event journal.

**Part of the armada vision** — layer L2 (the journal), the home the security
audit, the communication bus, the workspace events, and the control plane's
tracing all ride. The plan is `docs/plans/2026-11-11-armada-l2-journal.md`.

- **The journal** — day-partitioned append-only JSONL with a global monotonic
  sequence number that survives restart; every event carries the install-id
  and, when there is one, the principal and session.
- **Durable named consumers** — a consumer persists a cursor and resumes
  exactly where it stopped; a lagged or restarted consumer never loses an
  event.
- **A responsible citizen** — a retention budget curtails old day-files by
  name, never a silent drop. The advisory calib stream and the session log
  are untouched; JSONL before SQLite, no new dependency.
- **The face hook** — `record()` journals every event additively beside the
  broadcast, so the seat/control/finished events that died on crash now
  survive. The stream is unchanged.
- **Surface** — `fleet journal` prints the trail, the last sequence number,
  and each consumer's cursor.

## v0.6.31 — 2026-11-11

The armada bedrock: fleet knows its own identity, and every run acts for a
principal.

**Part of the armada vision** — layer L1 (identity), the first brick of the
bottom-up build. The decomposition, choices, and contracts live in
`docs/plans/ARMADA-*.md`; the plan is
`docs/plans/2026-11-11-armada-l1-identity.md`.

- **The install-id** — every install mints a stable id once (`fleet install`,
  `<home>/install-id`), idempotent across re-runs; a fresh home is a fresh
  install. `fleet identity` prints it. The corpus cursor, the roster, and
  deployment will key on it.
- **The principal** — every run acts for a principal; lane 1 is the owner.
  The principal rides the run, and a delegated child inherits its parent's,
  recorded on the child's start record so the run's authority is
  reconstructable from its log. Principal-agnostic from the first cut, so the
  org multiplies principals without code change.
- **Device→principal** — pairing links a device to the owner; an unlinked
  device resolves to no principal, never a silent fallback.
- **Surfaces** — `fleet identity`, the install mint, and the settings identity
  card beside the SOUL slot.

Carries identity only — grants nothing, refuses nothing; the uniform guard is
the next layer (L3). Single-owner behavior is unchanged.

## v0.6.30 — 2026-11-11

The first armada-vision increment: fleet pulls knowledge from other harnesses.

**Part of the armada vision** — the shared-mind layer's first working piece.
The direction in full is `docs/architecture/2026-11-11-armada-direction.md`.

- **The cross-harness memory pull** — `fleet memories --pull [--project <name>]
  [--source <harness>]` imports a project's facts from another installed
  harness (zcode first) into fleet's git-tracked corpus: provenance-tagged
  (`source: zcode:<project>:<revision>`), basis inferred, deduped against
  native facts (never clobbered), supersede-on-update, archive-on-delete. A
  one-time bootstrap plus a commit-cursor delta (git HEAD where tracked, a
  stable content hash otherwise). The `Source` abstraction leaves the armada
  remote-instance feeder a named stub. Proven on real data: 125 fresh facts
  imported, every native fact protected, the second pull unchanged.
- **The README, reframed** — around the why and the payoff: verifiable facts
  drive everything; work you can trust, a memory that compounds your
  knowledge, capability you govern, one productive front end. The substrate
  philosophy (memory, skills, tools, workflows, loops) and the methodology
  are front and center; self-containment is table stakes, not the pitch.
- **The armada direction note** — the security substrate (principals, grants,
  guards, step-up 2FA), the relay as the two-way command lane, the surface
  progression (CLI/TUI → PWA → Tauri), and the muxin / omni / orbit prior art
  it borrows from.

## v0.6.29 — 2026-11-11

The KI burndown: conversations are named and found by what they were, and the
scheduled corpus refresh stops fighting the fact-name grammar.

- **A conversation is named by its task, not its opener** (KI-480). The
  recall fact's task falls past house words ("warmup", "hi") to the first
  substantive owner message, and the digest now reads every owner message
  and compaction summary ahead of the tool calls — so a recall query finds a
  long conversation by its content words ("memory, cpu, disk", "good
  citizen"), not only by its calls. The sessions rail titles a row by the
  run's own summary when the first message is a greeting.
- **The corpus refresh names the fact dotless** (KI-466). The scheduled
  task string now instructs the fact name `corpus-<project>-<date>` (the
  remember grammar refuses a dot) and explains that the substrate writes it
  to the file `corpus-<project>-<date>.md`, which the tree's ignore rule
  matches — so the refresh no longer hits the one-call refusal every window.
- **The plan-gate reads criteria under the right heading** (KI-456, in
  dev-decisions). The gate's criteria parser is scoped to the Acceptance
  criteria heading, so a plan's Use-cases block no longer shifts every
  criterion's id; the plan that parsed 19 now parses its declared 14.

## v0.6.28 — 2026-11-11

Two active sessions at once, and the face never wedges behind a phantom run.

- **The face runs admitted runs concurrently, one per session** (the
  `docs/plans/2026-11-11-multi-session.md` plan, closing KI-525). The single
  serial command loop is now a dispatcher: each admitted run drives its own
  session on its own thread, with its own event forwarder so two transcripts
  never cross. The per-session state table that already existed is now the
  source of truth for execution, not just display.
- **The wedge is gone.** KI-525 was a stale `running=true` on an empty chat
  whose `driving` named a dead run — a second send queued behind work that
  never executed, blocking the composer. A restart now reconstructs an idle
  face, a refused run clears its own flag, and `driving` hands back to a
  running peer rather than naming a finished one.
- **Concurrency follows the machine's capacity, politely.** Before a second
  run starts, `contract::admits_concurrent` grades the cumulative run budget
  against the volume's headroom above the reserve (the resource contract's
  own strict arithmetic). Within capacity it admits; beyond it, the face
  paints a courteous named card telling the owner the run needs to be
  reconsidered or queued — never a silent over-spend, never a terse refusal.
- **The health card's line set is stable across ticks** (KI-513). A fast pass
  carries the previous pass's slow-tier verdicts beside its own fresh ones
  (`HealthReport::merged_over`), so a down docker/lanes/release service is no
  longer invisible on every other tick; `carried` names which lines were not
  re-probed.

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
