---
status: completed
created: 2026-10-30
updated: 2026-10-30
slug: surface-curation
---

# Plan: the curator — the memory and tool surfaces decided per turn, not per session

## Context

The owner's question, 2026-10-30: whether lanes for memories and tools
— a `mem1` and a `tool1` — would improve decision making by carrying
*"the most relevant and available memories and tools to use at any given
time during the session"*, following the patterns the app already uses.

The deep dive that preceded this plan found the answer is not "no" but
"the pattern belongs somewhere else." Both decisions the owner wants
already exist and are well built. Both are made once, at the wrong
moment; one never expires; and neither models availability.

**The memory decision already exists and is a genuine hybrid ranker** —
six weighted channels: vector cosine 0.15, keyword coverage 0.15,
trajectory phrase coverage **0.30** (the largest, deliberately: a fact's
recorded phrases are its direction), recall history 0.25, 90-day
freshness decay 0.10, provenance 0.05 (`src/memory.rs:1265-1296`). It is
shape-partitioned (the instruction's shape picks the primary pool), capped
at `ASSEMBLY_TOKEN_BUDGET = 2048` (`src/memory.rs:3002`), floors noise at
`MIN_RECALL_SCORE = 0.20` (`src/memory.rs:1368`), and never evicts the
protected tier.

**The tool decision already exists** — a count/token tier
(`MAX_VISIBLE_TOOLS = 32`, `MAX_DEFINITION_TOKENS = 9_000`,
`src/agent.rs:151-166`) above which lane tools are held behind
`tool_search`, and a relevance ranker over those deferred tools (sys1
class + sem1 vectors, `FLOOR = 0.68`, `MARGIN = 0.02`,
`ACTIVATE_MAX = 5`, `src/tool_router.rs:13-27`).

The mechanism is not the gap. These four are, each measured today:

1. **Memory is frozen at turn one.** `prepare_memory(task)` runs at
   `src/agent.rs:2255`, *before* the turn loop at `:2260`, against the
   **opening task only**, and is baked into the system prompt (`:2256`);
   every later turn reuses that string verbatim (`:2292-2303`). A steer
   (`:2846`) does not re-rank it. Compaction replays it unchanged
   (`:2681`). A session that opens on "fix the lane probe" and ends three
   hours later on memory ranking still carries the lane-probe facts. The
   only per-turn recompute is the model choosing to call `recall` — 8
   facts, read-only, model-initiated (`src/recall.rs:24-30`).
2. **Tool activation never expires.** `tool_search` inserts into
   `self.activated` (`src/agent.rs:4174`, `:4184`) and nothing removes it
   — every reference is an insert, a read, an init, or a declaration
   (`:1129`, `:1368`, `:1854`, `:5516`). The field's own comment says
   "the next request" (`:1127`) and the model is told so (`:4177`), but
   the set is run-scoped: five tools activated in turn 2 are still
   declared in turn 40.
3. **There is no availability model.** `LaneChild::Dead` deliberately
   keeps its tools declared for the rest of the run
   (`src/agent.rs:6555-6569`, returned by `lane_tools` at `:6645-6662`,
   refusing with a named reason at `:6834-6844`) so the log's
   `lane_tools` record stays consistent with what the model saw. The
   consequence: a lane that dies at turn 3 keeps advertising five tools
   that fail fast until the run ends, and the model cannot distinguish
   "available" from "declared."
4. **The class half of `tool_search` is dead.** `src/agent.rs:4061`
   calls `sys1_classify` with `task: "tool_route"`; the vendored sys1
   registers a task with id `"route"` and nothing named `tool_route`
   (`services/sys1/src/sys1/tasks.py:523`; `grep tool_route
   services/sys1/` returns nothing). The call cannot resolve, so
   `parse_capability` (`src/tool_router.rs:76`) yields no class and
   ranking proceeds blind — the exact "named fallback, never a guess"
   path at `src/agent.rs:4062-4070`, taken on every search. Note that
   renaming the call to `route` is **not** the fix: `route` answers
   `workload`/`execution` heads, while `parse_capability` reads a
   `capability` head.

Measured costs that bound the design: the native surface is ~6,500
tokens and ~8,034 with memory across 28 declarations
(`src/agent.rs:157-166`; `docs/plans/2026-10-23-tool-availability-gaps.md:657-661`);
the app's own architecture note cites the external evidence for pruning
— an 85% cut in definition-token share and selection accuracy 49% → 74%
(`docs/architecture/tool-surface.md:238-242`, and the 30–50 degradation
band). Lane spawn measured live today at 53–99 ms per lane (blend1,
which composes four lanes, at 721 ms).

## Decisions (the owner's, dated and quoted)

1. **2026-10-30 — the question.** *"what I would like to explore is if
   having lanes for memories and tools would improve the decision
   making? meaning a mem1 and tool1 that carry the most relevant and
   available memories and tools to use at any given time during the
   session, so that they follow the same patterns we are using across
   the app"* — the "at any given time during the session" clause is the
   load-bearing part, and it is the one thing the current design does
   not do.
2. **2026-10-30 — the shape.** *"yes, draft the plan"* — approving the
   proposed order: the in-place fixes first, the lane-shaped judgment
   last and gated on the fixes proving the labels useful. Nothing in
   this plan adds a lane process; §C4 records what would have to be true
   before one is worth building.

## Approach

**A curator, not two more services.** The app's house rule is
propose/dispose: sem1 states it — *"vectors in, vectors out — embeddings
propose, the decision layers dispose"* (`services/sem1/src/sem1/__init__.py:1-15`).
sys1 proposes a label and `route.rs` walks; sem1 proposes vectors and
`tool_router.rs` ranks. A `mem1`/`tool1` that returned "use these three"
would be a lane *disposing*, which no lane does today.

Four concrete reasons a process boundary is the wrong container here:

- **State duplication.** The 114 fact vectors live in the agent's
  `Memory`, loaded once and cached by digest in
  `memories/.recall-index.json`. A separate process would either reload
  the corpus on every spawn or have vectors shipped over JSON-RPC each
  turn. In-process re-curation reuses what is already in memory and
  costs exactly one instruction embed — the same cost the `recall` tool
  already pays (`src/memory.rs:2427`).
- **The 8901 load amplifier.** Every lane child that embeds is another
  caller on the single `llama-server` (`--parallel 1`). KI-325, KI-344
  and KI-346 are all saturation wedges on that endpoint. A per-turn
  embedding lane adds a caller to the pool that already starves;
  in-process curation adds none.
- **Prompt-cache churn.** The system prompt is byte-stable across a run
  today (only the repair variant swaps, `src/agent.rs:2292-2303`), which
  is what makes provider-side prefix caching work. Re-assembling memory
  *into the system prompt* every turn would make every request a miss.
- **Pure functions over held data.** `tool_router.rs` does no I/O at all
  (`src/tool_router.rs:1-7`). A process hop around a pure ranking over
  in-memory vectors buys latency, not isolation.

**The design rules the plan commits to:**

- **Re-curate on a trigger, never on a tick.** Two triggers: *topic
  drift* (cosine between successive user/steer turns — the app already
  measures 0.58–0.66 unrelated vs 0.77–0.93 related,
  `src/gui/related.rs:28-31`) and *tool-result failure* (a lane call that
  came back an error re-curates the surface). Re-curating every turn is
  waste.
- **Deliver at the tail, never in the prefix.** A curated memory note
  rides at the end of the messages for the turn it was computed for. The
  system prompt stays byte-identical — pinned by a test, not a comment.
- **Cache the verdict by hash.** The router's own pattern
  (`src/route_cache.rs`): `hash16(curation text + surface version)` →
  verdict, so re-curation of an unchanged situation is free.
- **Expiry is part of activation.** An activated tool lives N turns and
  then revokes; a call re-activates it.
- **Availability is a transition, not a filter.** A lane going `Dead`
  emits a log record and *then* stops declaring — the record is the only
  reason the tools were ever kept.

**Where the lane shape genuinely fits**, and why it is last: sys1's
contract — a closed-set label with confidence, over text, fail-open — is
exactly a curation judgment. The class vocabulary already exists
(`read|search|edit|execute|web|memory|vision|media|delegate`,
`src/tool_router.rs:285-310`). Registering a `tool_route` task in sys1 is
in C1 regardless, because rule 4 above is a live defect. Whether the
curator then *asks sys1 for a label* per turn is a separate question with
its own entry criteria (§C4).

## Increments

Each lands whole — code, checks, docs, its KI row — and each is green on
its own.

### C0 — availability: a dead lane stops declaring

`LaneChild::Dead` stops contributing to `declarations()` while keeping
its tools callable-with-a-named-refusal. `lane_tools()` keeps its current
shape; `declarations()` filters on state.

Reconstruction (hard rule 5) is the constraint, and the record already
exists: `Record::LaneFailed { lane, reason }` (`src/session.rs:146`) is
written today for a lane that failed to join at run start
(`src/agent.rs:2227`), while `Record::LaneTools` records the tools a lane
brought (`:2223`, `:3943`). The minimal change is **reuse, not a new
variant**: widen `LaneFailed`'s doc from "did not join at run start" to
any death, write it on the `Dead` transition, and teach the log→lane
reconstruction (`src/agent.rs:1601`, `src/gui/bridge.rs:5048`,
`src/tui/app.rs:3114`) one rule — a `LaneFailed` after a `LaneTools` for
the same lane withdraws that lane's tools. Those readers are the replayers
that would otherwise still show the dead lane's tools.

*Checks:* K1.

### C1 — activation expires, and the class actually arrives

Two halves of one surface, because activation is what `tool_search`
produces:

- `self.activated` becomes turn-stamped (`BTreeMap<String, u64>`);
  `tier_declarations` ignores entries older than `ACTIVATION_TURNS`
  (start at 3, measured in C3). A call to a lapsed tool re-activates it.
- Register a `tool_route` task in the vendored sys1 whose heads carry
  `capability` over the nine-class closed set, with `none` as a real
  answer. This is not a rename to `route` — `route` answers
  `workload`/`execution` and would leave `parse_capability` blind.

*Checks:* K2, K3.

### C2 — the memory half: re-curate on drift

A `curate_memory(task)` step in the turn loop, called when the drift
trigger fires (and on the first steer, which is a drift by definition —
`apply_steer`, `src/agent.rs:2846`). It calls the existing
`memory.assemble(task)` against the *current* text, renders with
`render_assembly`, and appends it as a turn-scoped note at the tail of
the request messages. The verbatim system prompt is untouched. Recall
hits are recorded once per curation, not once per turn (KI-068's rule,
adapted).

The record needs a decision this plan hands the implementer, with its
constraint named: `Record::Memory { shape, facts }` exists
(`src/session.rs:236`) but is written **once**, at assembly, and means
"the block in the system prompt" — the tests read the first one with
`.find()` (`src/agent.rs:8626`, `:8736`) and the face appends them
(`src/gui/bridge.rs:5063`). A curation at turn *n* is a different thing
from the opening block, so either `Memory` gains a turn stamp or a
distinct variant carries it; whichever is chosen, a replay must place the
curated set at the turn it was delivered, and the existing `.find()`
readers must keep meaning "the opening block."

*Checks:* K4, K5.

### C3 — the ranker over core tools, and the measurement

`tool_router`'s ranking is extended to the full declaration set, not just
deferred lane tools, so a narrowed session can withhold core tools it
cannot plausibly need. Two guardrails: the built-in `Tool` enum stays
declare-by-default unless the surface is over budget, and any withholding
is recorded (the log names what was hidden and why). This is the
increment that pays for itself in tokens — the definition-token win is on
the core surface, not the lane surface. The measurement quotes before and
after declaration tokens and tool count on a real run.

*Checks:* K6, K7.

### C4 — deferred: the curation judgment as a lane

**Not built in this plan.** Named with entry criteria so the decision is
made on evidence, not appetite. Build it only if C2+C3 show the drift
trigger mis-firing on real sessions (a label would beat a cosine floor),
or if the owner wants to swap the salience *policy* without rebuilding —
the reason sys1 is a lane at all. If built: a `curate` task in sys1
answering `{focus, needs_recall, staleness}` with confidence, cached by
turn hash exactly as `route` is cached, with `route.rs` still owning the
selection. It would be sys1's shape, not a new process.

**Landed follow-up:** the owner took criterion (b) on 2026-10-30 — the
plan for the deferred curator is written at
`docs/plans/2026-10-30-curator-judgment.md` (status draft, awaiting
implementation), with the entry-criteria status named honestly: the
cosine floor mis-fires on zero real pairs (the K5 replay), so the
judgment buys policy — early fires, focus naming, swappability — and
must earn the deciding seat on the replayed evidence or ship advisory.

## Use cases

- [x] A: A session that changes subject by turn 20 carries the memories
      for the new subject, not the opening one.
- [x] B: A lane dies mid-run and the model stops seeing its tools, with
      the withdrawal named in the log.
- [x] C: A tool activated for one question is not carried for fifty
      turns.
- [x] D: A session that narrows to reading files declares fewer tools,
      and the log says which were withheld.

## Acceptance criteria

- [x] **K1 — a dead lane stops declaring.** A lane that goes `Dead`
      contributes nothing to `declarations()`; a call to one of its
      tools still returns the named refusal; a `LaneFailed` record names
      the lane and the reason, and every reader that reconstructs the
      tool set from the log treats a `LaneFailed` after a `LaneTools` for
      the same lane as withdrawing that lane's tools. Pinned by a test
      that kills a lane mid-run and reads the declarations, the refusal,
      and the reconstructed set.
- [x] **K2 — activation expires.** A tool activated at turn *n* is absent
      from the declarations at turn *n + ACTIVATION_TURNS*; calling it at
      *n + 1* re-activates it through *n + 1 + ACTIVATION_TURNS*. Pinned
      by a unit test over `tier_declarations` with turn-stamped entries.
- [x] **K3 — the class arrives.** A `tool_search` call against the
      vendored sys1 returns a class from the nine-class set (or `none`),
      and the ranking reflects it; a sys1 that answers nothing still
      yields the named "ranked without the class" fallback. Pinned by a
      live lane call in the integration tests plus a scripted-child
      case.
- [x] **K4 — memory re-curates on drift, and the prefix is pinned.** A
      steer mid-run produces a curated note at the message tail and the
      system message is byte-identical to turn 1's. Pinned by a test that
      hashes the system message across turns.
- [x] **K5 — the trigger is measured, not vibes.** `DRIFT_FLOOR` is
      chosen from measured pairs (the `gui/related` bands, plus at least
      one real session replayed) and recorded here; the same input
      produces the same trigger decision.
- [x] **K6 — core tools can be withheld.** Over budget, a core tool that
      scores below the floor is withheld and named in the log; the
      built-in `Tool` enum is unchanged and the surface never withholds
      a tool the run cannot proceed without (`edit` without `read`,
      etc. — held as an explicit allowlist).
- [x] **K7 — the measurement.** Before/after declaration tokens and tool
      count on one real run, quoted in this file; the native surface
      stays inside `MAX_DEFINITION_TOKENS`.
- [x] **K8 — the gate and the docs.** fmt, clippy, build, test green by
      exit code; the feature docs updated; the KI rows filed in the
      commits that close them.

## Files to be touched

**Backend:**
- `src/agent.rs` — turn loop curation call (C2); `activated` as
  turn-stamped (C1); `declarations()`/`tier_declarations` filter (C0,
  C1, C3); `LaneRoster::lane_tools` state handling (C0); the drift
  trigger
- `src/gui/bridge.rs`, `src/tui/app.rs` — the `LaneTools`/`LaneFailed`
  replay readers learn the withdrawal rule (C0)
- `src/tool_router.rs` — ranking over the full declaration set (C3); the
  turn-expiry helper lives here so the rule is testable without the loop
- `src/memory.rs` — a `curate`-shaped entry beside `assemble` that takes
  the current text and returns a rendered note (thin wrapper; the ranker
  does not change)
- `src/session.rs` — `LaneFailed`'s doc widened to any death (C0); the
  `Memory` turn stamp or the curated-block variant (C2), whichever the
  reconstruction readers make cheap
- `src/route_cache.rs` — reused for curation verdicts if C2 grows a
  cached judgment; no change in this plan's scope

**Vendored lane:**
- `services/sys1/src/sys1/tasks.py` — the `tool_route` task (C1)
- `services/sys1/VENDOR.json` — the fleet-addition note, if the task is
  a fleet addition rather than an upstream one

**Tests:**
- `src/agent.rs` module tests — K1, K2, K4, K6
- `tests/*.rs` integration — K3 against a live sys1, the scripted-child
  case
- `tests/face/harness.mjs` — only if a surface shows the curation, which
  this plan does not add

**Documentation:**
- `docs/features/tools.md` — the tier's real expiry rule
- `docs/features/memory.md` — per-turn curation beside the assembly
- `TECHNICAL-DOCUMENTATION.md` — the curator's module reference
- `docs/known-issues.md` — the three rows below

## Out of scope

- **`mem1` and `tool1` as new lane processes** — the answer to the
  owner's question, and the reason §C4 exists as a gated follow-up.
- **A second embedding backend or a second `llama-server`** — the
  contention rows (KI-325, KI-344, KI-346) are the reason curation stays
  in-process.
- **Changing the six memory weights** — re-curation changes the
  *instruction*, not the ranker. The weights are calibrated and stay.
- **Re-curating on every turn** — deliberately rejected; the triggers
  are drift and failure.
- **Skill selection as a third surface** — the same curator could serve
  `use_skill`; named as a possible follow-up, not built.
- **Any new UI** — the TUI and the face show what they show today. A
  curation row in the pane is its own decision.
- **The vision-workload defect and the `measure`/TABPFN key starvation**
  found in the same dive — separate, already diagnosed, not covered here.

## Verification

**K1** — the declaration set after a mid-run lane death:
```bash
cargo test a_dead_lane_stops_declaring
```
Expected: green; the refusal text and the withdrawal record both named.

**K2** — activation lifetime:
```bash
cargo test an_activation_expires_after_its_turns
cargo test a_lapsed_tool_reactivates_on_call
```
Expected: green.

**K3** — against the live lane:
```bash
cargo test tool_search_names_a_capability_class
```
Expected: green, with the class quoted in the test's output. Manually, the
same call through the lane:
```
tools/call {"name":"sys1_classify","arguments":{"task":"tool_route","text":"find where retries are counted"}}
```
Expected: an `answers.*.capability.label` of `search` (or another of the
nine), where today it is an error.

**K4** — the prefix pin:
```bash
cargo test the_system_prompt_is_stable_across_turns
```
Expected: green; the curated note appears in the message tail only.

**K6/K7** — the measurement, on one real run, quoting declaration tokens
before and after and the tool count; the gate run whole:
```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo build && cargo test
```
Expected: exit 0 each.

## Linked artifacts

- `docs/architecture/tool-surface.md` — the external evidence the tier
  rests on (30–50 band, 85% token share, 49%→74%); C3 is the increment
  that finally applies it to the core surface
- `docs/plans/2026-10-23-tool-surface-and-lane-resolution.md` — the
  tier's origin; C1 corrects its activation lifetime
- `docs/plans/2026-10-15-route-decision-cache.md` — the hash-keyed
  verdict cache C2/C4 borrow
- `docs/features/memory.md` — the six-channel ranker this plan reuses
  rather than replaces

## Risks

- **The curated note leaks into the system prompt and kills prefix
  caching.** Mitigation: K4 pins the system message byte-for-byte;
  delivery at the tail is a rule, not a preference.
- **Dropping dead-lane declarations breaks prompt reconstruction.**
  Mitigation: K1 requires the withdrawal record in the same commit —
  the record replaces what keeping the declarations was buying.
- **The drift floor is mis-set and re-curation becomes either noise or
  never.** Mitigation: K5 requires a measured floor and a deterministic
  decision; C3's measurement revisits it on a real session.
- **Withholding a core tool strands the model mid-task.** Mitigation: K6
  requires an explicit dependency allowlist and that withholding is
  recorded; the surface never withholds a tool whose prerequisite is
  present (a write without a read).
- **An extra embed per curation.** Mitigation: curation is trigger-bound
  and reuses cached fact vectors; it costs one instruction embed, the
  same as today's `recall` tool.
- **The `tool_route` task is a fleet addition to a vendored lane.**
  Mitigation: `VENDOR.json` records it as a `fleetAddition` (the pattern
  blend1 already uses), so a re-vendor does not silently drop it.

## Dependencies

- The router/lane deep dive of 2026-10-30 — the diagnoses in Context,
  each with file:line.
- KI-325 / KI-344 / KI-346 — the `llama-server` contention rows that
  keep curation in-process.
- The `route_cache` module — the verdict-cache pattern C4 would reuse.

## Notes

- **The three defects this plan closes, for the ledger.** Read the
  next free numbers at landing time; the ledger's tail was KI-374 when
  this was drafted, and `.lock` files suggest concurrent agents may take
  them:
  1. *A dead lane's tools stay declared for the rest of the run* —
     `src/agent.rs:6555-6569`, `:6645-6662` — fixed by C0.
  2. *`tool_search` activations never expire, contradicting their own
     documented "next request"* — `src/agent.rs:1127`, `:4174`,
     `:4184` — fixed by C1.
  3. *`tool_route` is not a registered sys1 task, so the class half of
     `tool_search` is dead on every call* — `src/agent.rs:4061` vs
     `services/sys1/src/sys1/tasks.py:523` — fixed by C1.
- **Why "per turn" is the owner's phrase and not the design's.** The
  plan delivers the owner's "at any given time" by making the *state*
  current at any given time, while the *recompute* stays trigger-bound.
  Per-turn recompute would be paying for a fresher answer than the
  conversation actually changed.
- **The measured lane-spawn numbers** (53–99 ms per lane, blend1 721 ms)
  are from a live `fleet lanes` run on 2026-10-30 and are the basis for
  "the hop buys latency, not isolation" in Approach.
- **Adjacent findings not covered here:** the vision workload always
  errors (`Client::routed` + the sync `Client::chat`, which has no
  router branch — `src/api.rs:603` vs `:628`), and `measure` is starved
  of `TABPFN_API_KEY` while blend1's tabular leg works (the two key
  universes). Both are diagnosed and recorded separately.

## Landed 2026-10-30

C0: the withdrawal is a logged transition — the `Live → Dead` death
writes one `Record::LaneFailed` (widened from "did not join at run
start" to any death; the doc names the rule), `declarations()` reads the
new `declared_lane_tools()` (live only) while `lane_tools()` keeps the
log's shape, the refusal text says the tools were withdrawn, and the
three log→tool-set readers apply the rule: `reconstruct_document`
retains out the withdrawn lane, the face emits a `withdrawn` mark on
the lane card (the three render sites in `app.js` read it), and the
TUI's `entry_of` renders the death where the join rendered the count
(`LaneCard.withdrawn`). K1: `a_dead_lane_child_degrades_the_run_without_killing_it`
(28 tools post-death where it pinned 29 declared, the withdrawal record
after the join, the progress line),
`a_dead_lane_keeps_its_log_tools_but_declares_none`,
`a_lane_withdrawal_removes_the_lanes_tools_from_the_reconstruction`,
`a_withdrawn_lane_renders_the_death_where_the_join_rendered_the_count`.

C1: `activated` is a turn-stamped `BTreeMap<String, u64>`; an activation
rides `ACTIVATION_TURNS = 3` requests counting the one it was activated
on, the search result names the window, and `call_lane` re-stamps on
call. The vendored sys1 registers `tool_route` — one `capability`
choice head over the ranker's own nine-class set plus `none` — recorded
in `VENDOR.json`'s `fleetAdditions`. K2:
`an_activation_expires_after_its_turns`, `a_lapsed_tool_reactivates_on_call`.
K3: `the_real_sys1_lane_answers_the_tool_route_task` (a real lane child,
no providers configured in the test env, answering a classify envelope
for the task — the exact call that raised KeyError before) plus the
scripted `tool_search_asks_sys1_and_ranks_with_sem1_end_to_end`.

C2: the drift trigger in the steer path. `Memory::instruction_vector`
exposes the recall embedder for measurement; `maybe_curate` compares
the steer against the last curated instruction's vector, the first
steer curating by definition and a keyword-only corpus treating every
steer as drifted. `DRIFT_FLOOR = 0.78`, measured live 2026-10-30 on
this machine's embeddinggemma over steer-shaped pairs: unrelated
0.647–0.729, paraphrases 0.846–0.925 — steer pairs read higher than the
`related` surface's fact-to-fact bands (KI-058: 0.58–0.66) because two
imperative asks share a register, so the floor takes the measured gap's
midpoint. The note rides the request's message tail as a system message
(never the system prompt — `the_system_prompt` pin below), turn-scoped,
recorded as `Record::Curated { turn, shape, facts }` (a new variant,
not a `Memory` stamp — the existing `.find()` readers keep meaning "the
opening block"). Recall hits re-arm per curation. K4:
`a_drifted_steer_recurates_memory_at_the_message_tail_and_the_prompt_holds`
(the system message byte-identical across all three requests, the note
at the first request's tail only, the steer as the owner's message, the
record at turn 1) and `the_first_steer_curates_and_the_record_lands_once`.
K5: `the_drift_floor_decides_deterministically`, **and the real-session
replay the criterion names**: the steer sequence of real session
`20261028T162708Z-997a54b4` (four control-file steers, read from
`~/.fleet/sessions`, frame-stripped) replayed against the production
embedder at 8901 through `maybe_curate`'s exact algorithm — opening
task as the base, the base advancing to each steer whatever the
verdict. The decisions: steer 1 fires (definition), steers 2 and 3
**hold** (cosine 0.882 and 0.884 — both continue the same
router-discussion thread, and holding is right), steer 9 **fires**
(cosine 0.689 against steer 3 — it pivots to a genuinely new
instruction, and firing is right). Zero mis-fires on the real pairs,
and the decision set is identical at floors 0.72, 0.78 and 0.84 — the
real data's gap (0.689 vs 0.882) brackets 0.78 with margin on both
sides, so the floor is not knife-edge. One honest observation recorded
for whatever later work touches the trigger: the session's plain
(non-steer) owner turns — "continue", "create a plan to fix both
issues" — embed low (0.644–0.701) against longer bases because short
imperatives lose to length; they never reach `maybe_curate` today
(only steers do), but a chat-mode curation path would need
length-normalized comparison before trusting this floor on them.

C3: `withhold_core` — the workload profile's rank is the score, the
explicit `CORE_KEEP` list (the eight working tools, the capability
trio, the memory pair, `subagent`, `process`, `use_skill`) is the floor
the score cannot cross, and the withheld names ride the `declarations`
record's new `withheld` field (serde-default, so old logs load). The
tier defers lanes first and reaches for core withholding only when even
the deferred shape cannot fit. K6: `an_over_budget_core_withholds_its_unranked_tail_and_names_them`,
`the_tier_withholds_the_core_only_when_even_the_deferred_lanes_cannot_fit_it`.
K7, the measurement on the real surface: **28 declarations, 7,293
definition tokens, 0 withheld** (`the_real_native_surface_fits_and_withholds_nothing`)
— the mechanism is dormant on today's surface, which is the point: a
fitting surface withholds nothing, and the record is the before/after
pair.

K8: the gate green by exit code (fmt, clippy, build, test — the counts
in the day's commit); the docs updated (features/tools.md's tier
section, features/memory.md's new "Mid-run curation" section,
TECHNICAL-DOCUMENTATION.md's `agent.rs` entry); KI-375, KI-376 and
KI-377 filed and closed in this landing.

The plan's C4 stands deferred, untouched, with its entry criteria
unchanged.
