---
status: completed
created: 2026-10-30
updated: 2026-10-30
slug: curator-judgment
---

# Plan: the curator's judgment — the deferred C4 as a lane answer, not a lane process

## Context

The surface-curation plan (2026-10-30, landed `a7007a4`) answered the
owner's mem1/tool1 question with a curator: the memory and tool surfaces
re-decided per turn, in-process, on triggers. Its C4 — *the curation
judgment as a lane* — was deferred behind entry criteria, and the owner
has now instructed: *"once completed create a plan for the deferred
curator."*

The landed plan's own evidence frames what this plan is and is not:

- **Entry criterion (a) — the drift trigger mis-firing — is NOT met.**
  The K5 replay ran real session `20261028T162708Z-997a54b4`'s four
  control-file steers against the production embedder through
  `maybe_curate`'s exact algorithm: zero mis-fires, the two on-topic
  steers holding at 0.882/0.884, the genuine pivot firing at 0.689, and
  the decision set identical across floors 0.72–0.84. The cosine floor
  works on the real data.
- **Entry criterion (b) — the owner wants the salience policy
  swappable — is the one met.** This plan exists because the owner
  instructed it. Its honesty test is therefore not "does the cosine
  mis-fire" (it does not) but **"does a judged curation beat a
  cosine-only one on the same real sequence"** — and the plan is built
  so the lane must *earn* the deciding seat or ship advisory.

What the cosine floor cannot do, by construction, is what a judgment
can:

1. **Fire early.** The cosine needs two vectors to disagree; a steer
   that says "stop — the premise is wrong" mid-thread scores *high*
   against the thread it contradicts (same register, same words, wrong
   direction). A judgment reads the move, not the distance.
2. **Name the focus.** The re-curation's shape partition runs
   `classify_shape` (`src/memory.rs:2080`) — keyword substring matching
   over a closed vocabulary. A steer that names no shape keyword gets
   the surface-recall fallback even when the owner's intent maps cleanly
   onto one. A judged `focus` head names the shape the words did not.
3. **Carry a policy that can be swapped without a rebuild** — the
   reason sys1 is a lane at all. The floor is a constant compiled into
   the binary; the judgment is a task in a vendored lane the owner can
   re-vendor, re-word, or re-weight.

The substrate this plan stands on is all landed and measured: the drift
trigger (`maybe_curate`, `src/agent.rs:2969`) with its measured
`DRIFT_FLOOR = 0.78` (`src/agent.rs:188`), the turn-stamped activation
window, the `tool_route` task as the vendored-task precedent
(`services/sys1/VENDOR.json`'s `fleetAdditions`), the route cache's
producer-keyed verdict discipline (`src/route_cache.rs`), and the
KI-044 lesson that an unconfigured judgment lane must be skipped before
it is ever spawned (`sys1_judges`, `src/route.rs:1919`).

## Decisions (the owner's, dated and quoted)

1. **2026-10-30 — build the deferred curator.** *"once completed create
   a plan for the deferred curator"* — after the surface-curation plan
   landed with C4 explicitly deferred. The instruction is criterion (b)
   speaking: the salience policy becomes a lane answer.
2. **The standing shape, from the landed plan's own C4 text:** *"a
   `curate` task in sys1 answering `{focus, needs_recall, staleness}`
   with confidence, cached by turn hash exactly as `route` is cached,
   with `route.rs` still owning the selection. It would be sys1's
   shape, not a new process."* This plan implements exactly that and
   nothing more.

## Approach

**One task in the vendored sys1; every byte of selection stays in
Rust.** The lane answers three heads over one call; `maybe_curate`
remains the executor and the cosine floor remains the fail-open. The
composition is a table, pinned row by row:

| situation | decision |
|---|---|
| the run's first steer | **fire** (unchanged — a drift by definition) |
| lane says `needs_recall: yes` at confidence ≥ the gate | **fire** — the upgrade: the judgment can fire before the cosine would |
| lane says `needs_recall: no` at confidence ≥ the gate | **hold**, unless the cosine is under the floor — the floor is the safety net, not the policy |
| lane low-confidence, unavailable, or no keys | **the cosine decides** — byte-identical to today's behavior |

The fail-open row is the plan's spine: a machine with no sys1 keys
never pays a spawn (the `sys1_judges` gate is checked before any lane
work, KI-044's lesson at `src/route.rs:1919`), and a run whose lane
answers nothing curates exactly as the landed plan built it. The
judgment is an upgrade that must prove itself (J5), never a dependency
the curation cannot live without.

**The three heads, over closed sets that already exist:**

- `needs_recall` — `yes | no`. Should the memory block be re-ranked for
  this instruction? The fire/hold vote.
- `focus` — the shape-partition class the re-curation routes under: the
  corpus's own **seven structure shapes** (`rendering`, `state-sync`,
  `deploy`, `build`, `change`, `deps`, `docs`, `src/memory.rs:1784`) and
  **six failure shapes** (`fail-open`, `drops-the-row`, `bound-exceeded`,
  `claim-outran-evidence`, `fixed-width-clip`, `foreign-root`,
  `src/memory.rs:1846`), plus `none`. The ranker's vocabulary, not a new
  one — a label the lane names is always one the partition can act on,
  the same rule `tool_route`'s label set followed.
- `staleness` — `fresh | stale`. Whether the block the model already
  holds is stale for this instruction, beside the fire/hold vote: the
  record's diagnostic, not a second trigger.

**The text contract**: the lane judges a *move*, so the input names
both ends — the instruction the block was last curated against, and the
new steer. A judgment that saw only the steer would be a topic
classifier; the pair is what makes "has the conversation moved" a
answerable question.

**The focus label acts, or it is dead weight.** When the lane names a
shape the keyword classifier missed, the re-curation's partition routes
under it: `Memory::assemble` gains an optional shape hint
(`assemble_with_shape`, a defaulting wrapper over `assemble`,
`src/memory.rs:3061`) and the hint wins over `classify_shape` when
present. When the lane names `none` (or the lane never answered), the
keyword classifier runs as today. The `Curated` record carries which
happened.

**The cache is the route cache's discipline, extended — not a second
file.** `route-cache.json` gains a second namespace: the same producer
key (`key_for`, `src/route_cache.rs:95`), a second map of
`hash16(steer + prior instruction)` → `CurateVerdict`. One file, one
rekey rule, old files load (serde default on the new map), and a
re-vendored lane invalidates both namespaces together — which is
correct, because both are judgments of the same producer. The cost
profile is the route cache's own: a judged steer costs one spawn plus
one classify (~200–600 ms hosted); a repeat steer with the same pair is
free; steers are rare by nature.

**The seam is honest about being async.** The lane call is awaited, so
`apply_steer` (`src/agent.rs:2946`) becomes async — it is already
called from the async turn loop (`src/agent.rs:2360`), and its only
other caller paths are the same loop's. The curation note's delivery
rule (the message tail, never the system prompt) is unchanged and
stays pinned by its test.

**Calibration rows make the upgrade gradeable.** Every judged curation
writes one calib row (`lane: "sys1"`, op `curate`) carrying the verdict,
the cosine, and the decision — so `fleet judge`'s machinery can grade
judged curations against their outcomes later, the way route rows are
graded today. The plan does not build the grader; it makes the rows
exist.

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### J0 — the task in the lane

`_CURATE` in `services/sys1/src/sys1/tasks.py`: three heads as above
(`needs_recall` yes/no, `focus` over the thirteen shapes + `none`,
`staleness` fresh/stale), with descriptions written from the corpus
vocabulary's own doc comments. The text contract documents the two-part
input. Recorded in `VENDOR.json`'s `fleetAdditions` — the pattern that
survives a re-vendor, exactly as `tool_route` did.

*Checks:* J1, J2.

### J1 — the verdict reaches the run, cached

`CurateVerdict { needs_recall, confidence, focus, staleness }` in
`agent.rs` beside `maybe_curate`, with the lane-answer parser. The
route cache gains the curate namespace (same producer key, second map,
serde-default load). `apply_steer` goes async and awaits the judgment
through the lane roster the way `run_tool_search` does, behind the
`sys1_judges` gate.

*Checks:* J3, J4.

### J2 — the composed decision, and the focus that acts

The four-row composition table above, in `maybe_curate`, with the
cosine floor unchanged as fail-open and safety net. The focus hint:
`assemble_with_shape`, the hint winning over `classify_shape` when the
lane named a shape. `Record::Curated` gains `trigger`
(`first-steer | cosine | judged`, serde default) and `focus_shape`.

*Checks:* J3 (the table, row by row), J4.

### J3 — the measurement that earns the seat, the rows, the docs

The replay: real session `20261028T162708Z-997a54b4`'s steer sequence
under judged mode versus the landed cosine-only mode, decisions side by
side in this file — plus any further steer-bearing session the archive
holds by then. Calib rows written per judged curation. The docs:
`docs/features/memory.md`'s mid-run curation section gains the
judgment; `TECHNICAL-DOCUMENTATION.md`'s `agent.rs` entry names it.
**The contingency, decided on evidence and recorded here:** if the
judged decisions do not dominate the cosine-only ones on the replayed
set (no new mis-fires, at least one genuine early fire or focus fix),
the composition's default flips to advisory — the lane votes, the
cosine decides — and the flip is recorded, not silently kept.

*Checks:* J5, J6, J7.

## Use cases

- [ ] A: A steer that contradicts the thread it lands in ("stop — the
      premise is wrong") re-curates even though its cosine against the
      thread is high. **Not delivered — by the contingency's own
      verdict.** The advisory seat means the cosine decides, and this
      use case is the deciding seat's promise: it lands when a future
      replay flips `CURATE_JUDGED_DECIDES` by the plan's bar.
- [x] B: A steer that names no shape keyword still routes its
      re-curation under the shape the lane named, and the record says
      the lane did it.
- [x] C: A machine with no sys1 keys runs the curation path exactly as
      the landed plan built it — no spawn, no latency, cosine decides.
- [x] D: A re-vendored sys1 invalidates the curate verdicts with the
      route verdicts, in one write.

## Acceptance criteria

- [x] **J1 — the task registers and answers.** A real sys1 child
      answers `sys1_classify` with `task: "curate"` as a classify
      envelope carrying all three heads; every label the lane names is
      inside the closed sets. Pinned live (the
      `the_real_sys1_lane_answers_the_tool_route_task` pattern) plus a
      scripted-child case.
- [x] **J2 — the addition survives a re-vendor.** `VENDOR.json`'s
      `fleetAdditions` names the task, its heads, and why; the doc
      comment in `tasks.py` says the same.
- [x] **J3 — the composition table is pinned, row by row.** Each row of
      the table is a unit test over `maybe_curate`'s decision with
      scripted lane answers and fixed vectors: first steer, confident
      yes, confident no with cosine above and below the floor,
      low-confidence, lane unavailable, no keys.
- [x] **J4 — the cache is one file, one rekey.** A curate verdict
      round-trips `route-cache.json`; a key mismatch clears both
      namespaces; a pre-judgment file loads with an empty curate map.
- [x] **J5 — the fail-open is byte-identical.** With `sys1_judges`
      false, the curation path performs no lane call and its decisions
      match the landed behavior — pinned by a test that strips the
      decision keys and replays a steer.
- [x] **J6 — the replay measurement is in this file.** The real steer
      sequence, judged versus cosine-only, decisions side by side; the
      contingency's verdict recorded with the numbers that decided it.
- [x] **J7 — the gate and the docs.** fmt, clippy, build, test green by
      exit code; the two docs updated; any defect found on the way is a
      ledger row closed in its own commit.

## Files to be touched

**Vendored lane:**
- `services/sys1/src/sys1/tasks.py` — the `_CURATE` task (J0)
- `services/sys1/VENDOR.json` — the `fleetAdditions` entry (J0)

**Backend:**
- `src/agent.rs` — `CurateVerdict` + the parser; `apply_steer` async;
  `maybe_curate`'s composition table; the `sys1_judges` gate check;
  `Record::Curated`'s new fields
- `src/route_cache.rs` — the curate namespace (second map, same key)
- `src/memory.rs` — `assemble_with_shape` (the hint wrapper)
- `src/calib.rs` — the `curate` op row (the lane's attribution shape,
  the `explain` row's precedent)

**Tests:**
- `src/agent.rs` module tests — J3's table rows, J5's fail-open
- `tests/integration.rs` — J1's live-lane case, the scripted case, the
  cache round-trip
- `src/route_cache.rs` module tests — J4

**Documentation:**
- `docs/features/memory.md` — the judgment beside the drift trigger
- `TECHNICAL-DOCUMENTATION.md` — the `agent.rs` entry
- `docs/known-issues.md` — rows closed in their own commits, as found

## Out of scope

- **A new lane process.** The judgment is a task in the sys1 lane that
  already exists; the owner's mem1/tool1 question was answered, and
  this plan does not reopen it.
- **The tool surface.** C3's withholding and the `tool_route` class are
  landed and untouched. If a judged focus ever feeds `profile_order`,
  that is its own plan.
- **Changing the six memory weights or `DRIFT_FLOOR`.** The floor is
  measured and validated on real pairs (0.647–0.729 unrelated,
  0.846–0.925 paraphrase, real-session replay stable 0.72–0.84); the
  judgment composes with it, it does not replace it.
- **The chat-mode curation path.** The short-imperative caveat recorded
  in the landed plan (plain owner turns embed low against longer
  bases) stands; chat-mode curation needs length-normalized comparison
  before any floor or judgment reaches it, and that is a separate
  decision.
- **The grader.** The calib rows exist so `fleet judge` *can* grade
  curations; building the grading pass is its own increment.
- **Any new UI.** The face and the TUI show what they show today.

## J6 — the replay measurement, and the contingency's verdict

Real session `20261028T162708Z-997a54b4`'s four steers, judged through
a real sys1 child (glide, the routed chain's head, keys from the fleet
home's dotenv) with the text contract's two-part pair, beside the
cosines the fail-open would have decided on:

| steer | cosine | needs_recall | confidence | focus | staleness | ms |
|---|---|---|---|---|---|---|
| 1 | 0.717 | yes | 0.174 | none | stale | 941 |
| 2 | 0.882 | no | 0.571 | deps | stale | 810 |
| 3 | 0.884 | no | 0.659 | none | fresh | 756 |
| 4 | 0.821 | yes | 0.536 | none | stale | 822 |

Side by side (the gate at 0.6, the floor at 0.78):

| steer | cosine-only | judged composition |
|---|---|---|
| 1 | FIRE (first-steer) | FIRE (first-steer) |
| 2 | HOLD (cosine) | HOLD (cosine — the vote sat under the gate) |
| 3 | HOLD (cosine) | HOLD (judged — the one vote over the gate, agreeing) |
| 4 | HOLD (cosine) | HOLD (cosine — the vote sat under the gate) |

**The verdict: identical decisions, and the contingency flips.** The
judged set has zero new mis-fires — but it does not dominate: no
genuine early fire (the lane's only `yes` votes landed at 0.174 and
0.536, both under the gate), and no focus fix (the one curation the
lane could have decided, steer 3, agrees with the cosine). Three of
four votes sat below the gate entirely — the lane, on the glide
provider, is under-confident for this task. The plan's own bar ("no new
mis-fires, at least one genuine early fire or focus fix") is not met,
so the default is **advisory**: the lane votes in the calibration
stream's `curate` rows, the cosine decides, and
`CURATE_JUDGED_DECIDES` (a named constant carrying these numbers in
its doc comment) is what a future replay flips — by the plan's bar,
not by appetite. The focus hint still acts on a fired curation: naming
the shape is the lane's least risky contribution, guarded by the
vocabulary check.

## Verification

**J1** — against the live lane:
```bash
cargo test --test integration the_real_sys1_lane_answers_the_curate_task
```
Expected: green; the envelope carries `needs_recall`, `focus`,
`staleness`, and every label inside the closed sets.

**J3/J5** — the table and the fail-open:
```bash
cargo test --lib the_curate_composition
cargo test --lib the_fail_open_curation_is_unchanged
```
Expected: green, one test per table row.

**J4** — the cache:
```bash
cargo test --lib curate_cache
```
Expected: green — round-trip, rekey, old-file load.

**J6** — the replay, re-run by hand against the archive and quoted in
this file:
```bash
# the same replay harness the K5 completion used, with the judged leg
# driven through a spawned sys1 child
```
Expected: the side-by-side table under J6's section in this file, and
the contingency's verdict written beside it.

**J7** — the gate, whole, exit codes read directly:
```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo build && cargo test
```
Expected: exit 0 each.

## Linked artifacts

- `docs/plans/2026-10-30-surface-curation.md` — the landed plan whose
  C4 this is; its Approach carries the propose/dispose rule, the
  8901-amplifier rule, and the tail-delivery rule this plan inherits
- `src/route_cache.rs` — the producer-keyed verdict discipline J4
  extends
- `src/route.rs:1919` — `sys1_judges`, the gate that makes J5's
  fail-open free
- `docs/features/memory.md` — the six-channel ranker and the mid-run
  curation section this plan extends

## Risks

- **The judgment mis-fires where the cosine held.** Mitigation: the
  composition's cosine safety net holds under every confident-`no` row;
  J5 pins the fail-open; J6's contingency flips the default to advisory
  on evidence rather than arguing.
- **A judged curation costs a spawn per steer.** Mitigation: steers are
  rare, the cache makes repeat pairs free, and the `sys1_judges` gate
  costs an unconfigured machine nothing. The landed plan's trigger rule
  (never a tick) is unchanged.
- **The focus hint routes the partition wrong.** Mitigation: the hint
  only wins when the lane is confident enough to have fired the
  curation at all, the keyword classifier remains the fallback, and the
  `Curated` record names which routed — so a wrong hint is diagnosable
  from the log.
- **A second namespace in one cache file drifts from the first.**
  Mitigation: the rekey discipline is shared code, not two
  implementations; J4 pins both namespaces clearing together.

## Dependencies

- The surface-curation plan (`2026-10-30-surface-curation.md`,
  completed, `a7007a4`) — the trigger, the floor, the delivery rule,
  and the `tool_route` precedent this plan builds beside.
- The K5 completion (`babb109`) — the real-session replay whose
  numbers set the entry-criteria status and whose harness J6 reuses.
- KI-044 (closed) — the unconfigured-lane lesson that becomes J5's
  fail-open pin.

## Notes

- **Why the plan's own evidence argues against urgency, and the owner
  built it anyway.** The replay shows the cosine floor making zero
  mis-fires on real steers; the honest reading is that the judgment
  buys *policy* (early fires, focus naming, swappability), not repair.
  That is exactly what criterion (b) names, and it is why J5's
  fail-open and J6's contingency are the plan's spine: the upgrade is
  welcome, it is not load-bearing, and it earns or loses the deciding
  seat on the replayed evidence.
- **The three heads are the landed C4 sketch's own** —
  `{focus, needs_recall, staleness}` — with the label sets chosen from
  vocabularies the ranker already acts on. No new taxonomy is
  introduced anywhere in this plan.
- **The calib `curate` row** follows the `explain` row's shape (a lane
  attributed op beside the run's own rows), so the judge's existing
  attribution machinery reads it without a schema change.

## Landed 2026-10-30

J0: `_CURATE` in `services/sys1/src/sys1/tasks.py` — three heads over
one call (`needs_recall` yes/no, `focus` over the corpus's seven
structure and six failure shapes + `none`, `staleness` fresh/stale),
the text contract a two-part pair, recorded in `VENDOR.json`'s
`fleetAdditions`. J1:
`the_real_sys1_lane_answers_the_curate_task` (a real lane child; every
label the lane names is inside the closed sets) and
`a_curate_answer_parses_into_a_verdict` (the scripted case, including
the named absence that feeds the fail-open row).

J1/J2: `CurateVerdict` in the route cache's second namespace — same
producer key, one rekey clears both maps, old files load (`a_curate_verdict_round_trips_and_an_old_file_loads_empty`,
`a_rekey_clears_both_namespaces`). `apply_steer` is async and awaits
the judgment; the `sys1_judges` gate is checked before any lane work.

J2: the composition table extracted as the pure `curate_decision`,
pinned row by row by `the_curate_composition_decides_row_by_row` — six
rows of the table plus the advisory collapse, all green.

J3: the calib `curate` row (`CurateRow` beside the `explain` row's
shape) written per judged decision — verdict, cosine, decision,
trigger — with the pair's hash as the join and the texts never stored.
The focus hint acts through `Memory::assemble_with_shape` over
`partition_with_shape`, the hint validated by `known_shape` and
recorded in `Record::Curated`'s new `trigger` and `focus_shape` fields
(serde-default; old logs load).

J5: the fail-open pinned at the composition level — the `judged: None`
rows ARE the no-keys machine's path, because the `sys1_judges` gate
answers before any lane work. Named limit, honestly recorded: the
gate is a process-wide `OnceLock`, so an in-process test cannot flip
it; the pin is the decision rows, not the env.

J6: the replay above; the contingency's verdict — **advisory** —
implemented as `CURATE_JUDGED_DECIDES = false` and pinned
(`!CURATE_JUDGED_DECIDES` asserted in the table test).

J7: the gate green by exit code — fmt 0, clippy 0, build 0, test
1,649 + 189, 0 failed; the two docs updated (features/memory.md's new
"The curation judgment" section, TECHNICAL-DOCUMENTATION.md's
`agent.rs` entry); no new ledger rows — the plan's work found none.
