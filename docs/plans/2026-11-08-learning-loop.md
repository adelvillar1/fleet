---
status: active
created: 2026-11-08
updated: 2026-11-08
slug: learning-loop
---

# Plan: the learning loop — Learning as a first-class work-menu feature

## Context

The LearnHarness deep dive and design
(`docs/architecture/deep-dive-2026-11-08-learnharness.md`) settled the
shape: learning is a first-class work-menu feature whose record rides
the memory substrate and whose friction feeds the self-improvement and
capability loops. This plan builds the whole feature in one pass — the
measure, the plan, the teaching, the knowledge graph and its
extraction, the practice, and the face surface — landing each phase as
its own commit and exercising it live before the next. The owner's
ethos: work fast, fail fast, fix fast — the phases are small, the
feedback is live, and a failure is caught by the next live run rather
than by a later plan.

Decision-model check: the probe is model-driven (the prober asks, the
learner answers); the capability-gap classification (the friction's
lack kind) is a rule over the friction's shape. No new decision seam.

## Direction lock

- **The substrate, not a silo.** The probe answers, the checks, the
  held propositions, the practice outcomes, and the lesson plans are
  facts and calibration rows — the recall, the dream pass, and the
  provenance apply unchanged.
- **The mockups first.** The Learning rail section and pane are a new
  surface: drawn, measured, read, and owner-read before the pane's
  code.
- **The learner's thinking is data.** The answers carry the reasoning,
  the uncertainty, and the confidence — record fields, not decoration.
- **The feeds land through their gates.** The friction's proposals go
  to the improve store, the drafter, the roles, and the cron — each
  through its own approval.
- **The evidence discipline.** Sessions are what happened; the graph
  is the intended map; confidence is self-reported; model-authored
  content can be wrong.

## Approach

The build rides the existing machinery: the session records, the
home-side roles, the graph module, the fact writer, the calibration
stream, the improve/dream/clustering consumers, and the face's rail.
Each phase lands as its own commit and is exercised live — the next
phase starts from the working one, and a failure is fixed in the open.

0. **The mockup (C0).** `docs/plans/mockups/2026-11-08-learning/`:
   the Learning rail section and its pane — the active lesson (the
   plan's coarse view, the current position), the graph, the practice
   entry, the history — drawn in the app's own tokens, measured in
   every theme, the PNGs read in a light and a dark theme, the owner
   reads it before the pane's code.
1. **The host and the measure (C1, C2).** `src/session.rs`: the
   `Start` record carries the lesson kind. A home-side `prober` role
   and `src/lesson.rs`: the probe loop — the questions through the
   conversation, each answer captured with its thinking and confidence,
   the current state bracketed, the goal pinned.
2. **The plan (C3).** The minimal tree in the graph module
   (`src/graph.rs`): the prober's bracketed states in, the DAG out —
   the nodes the propositions to teach, the edges the single-step
   dependencies, the critical path the teaching order; the coarse view
   is what the learner sees.
3. **The teaching loop (C4).** The teacher role and the lesson flow:
   one reasoning step at a time — predict, explain, check; confusions
   repaired before continuing; the goal's self-evidence the terminal
   condition, read from the learner's own words.
4. **The knowledge graph (C5).** The propositions and chains as facts
   (the `proposition` type, the `premises` field validated through
   `Remember::parse`); the extraction pipeline (extract → audit →
   merge) over the teaching prose, gated by the uncovered-token
   threshold; the read model over the substrate.
5. **The practice (C6).** The practitioner role and the problem
   sessions: varied problems requiring the knowledge to come to mind
   unnamed; one active problem per turn; the outcomes as session
   evidence, never graph mastery.
6. **The face surface (C7).** The Learning rail section and pane (fed
   by the mockup): the active lesson, the graph, the practice entry,
   the history.
7. **The feeds (C8, C9).** The learning events join the calibration
   stream as rows; the demand map's classes include the learning
   rows' confusions; the capability gaps land through the improve
   store and the drafter; the recurring confusions feed the
   clustering beat's shape proposals.
8. **The gate (C10).** `cargo fmt --all --check`, `cargo clippy
   --all-targets -- -D warnings`, `cargo build --release`,
   `cargo test` — each exit code read directly, the whole suite, run
   before the push.
9. **The docs (C11).** `docs/features/learning.md` (new), the
   surfaces doc's rail inventory, and the memory doc's proposition
   type and premises field.

## Use cases

- [ ] A: A learner starts a lesson — the probe brackets what they
  hold and pins the goal; the plan draws the minimal tree; the
  teaching moves one checked step at a time; the goal feels obviously
  true.
- [ ] B: The graph accumulates — the propositions and chains recall
  like project knowledge in later sessions.
- [ ] C: The friction feeds the loops — a confusion names a missing
  doc (a proposal), a recurring procedure (a skill draft), a needed
  expertise (a persona), a repeated workflow (a cron).
- [ ] D: Practice exposes a gap — the outcomes are session evidence,
  and the capability feed proposes the remedy.

The use cases map to the phases: A (the lesson end to end) runs
phases 1–4; B (the graph recalls) is phase 4; C (the friction feeds)
is phase 7; D (the practice gap) is phase 6.

## UX routes

The Learning rail section and pane (the mockup's surface): the rail
places it beside the other work surfaces; the pane shows the active
lesson, the graph, the practice entry, and the history. The lesson
itself runs in the conversation surface; the practice problems arrive
as the problem sessions' turns.

## Acceptance criteria

- [ ] **C0 — the mockup.** `docs/plans/mockups/2026-11-08-learning/`
      holds the Learning rail section and pane in the app's own
      tokens, measured in every theme (no overflow, ink ≥ 4.5:1), the
      PNGs read in a light and a dark theme, and the owner reads it
      before the pane's code lands.
- [x] **C1 — the lesson session kind.** `src/session.rs`'s `Start`
      record carries the kind; the session list and the face's
      session surface filter by it; pinned by `src/session.rs` tests.
      *Landed 50b380f: `Start.kind` carries `teaching`/`practice`,
      `Session::create_lesson`, `Session::list_kind`, `fleet sessions
      --kind`, and the face's sessions read `?kind=`; the session tests
      pin the kind's survival and the filter's exactness.*
- [x] **C2 — the prober and the probe loop.** The `prober` role under
      `<home>/roles/` and `src/lesson.rs`'s probe loop ask the
      learner, capture each answer with its thinking and confidence,
      bracket the current state, and pin the goal; pinned by
      `src/lesson.rs` tests with the scripted reply.
      *Landed 50b380f: `seed_prober` (never overwriting an owner's
      file), the `ProbeConversation` seam with `probe_loop` asking one
      question at a time, and `record_probe` landing the answers and the
      bracket as `probe` facts plus one `learn_probe` row.*
- [x] **C3 — the plan.** The minimal tree builds in `src/graph.rs`
      from the bracketed states: the nodes, the single-step
      dependencies, the critical path as the teaching order; the
      coarse view is the learner's; pinned by `src/graph.rs` tests.
      *Landed d4516f2: `LessonNode`/`LessonPlan`/`build_lesson_plan`
      (refusing a premise naming nothing, a goal naming no node, a
      cycle, an off-path proposition, a blank, a duplicate), the
      critical path as the teaching order, the Mermaid coarse view, and
      the plan's `lesson-plan` fact.*
- [x] **C4 — the teaching loop.** The teacher role and the lesson
      flow move one reasoning step at a time — predict, explain,
      check — with the confusions repaired before continuing and the
      goal's self-evidence the terminal condition; pinned by
      `src/lesson.rs` tests.
      *Landed e063b4f: the teacher role's seed, the
      `TeachingConversation` seam with `teach` (the foundations never
      re-taught, the repairs bounded, an unrepaired confusion stops the
      lesson, and the learner's own "obviously true" is the only
      terminal condition), and `record_lesson` landing one `check`
      fact per step plus the lesson's own fact. `teaching_order` reads
      the critical path as the spine, foundation first.*
- [x] **C5 — the knowledge graph.** The propositions and chains land
      as facts (the `proposition` type, the `premises` field through
      `Remember::parse`); the extraction pipeline runs over the
      teaching prose, gated; the recall and the dream pass apply;
      pinned by `src/memory.rs` and `src/lesson.rs` tests.
      *Landed: the `premises` field rides the write path's own
      validator (a premise that is not a proposition name is refused),
      the frontmatter, the loader, the edit, and the supersede's
      inherit-or-clear rule; `record_proposition` writes the
      `proposition` type through `Remember::parse`; the extraction
      pipeline audits the teaching prose against the claims and merges
      at the 80% uncovered-token gate, deterministically (the ordered
      union of premises, the first writer's claim standing); the
      memory tests pin the round-trip and the lesson tests pin the
      recall, the gate's refusal, and the merge.*
- [x] **C6 — the practice.** The practitioner role and the problem
      sessions pose varied problems requiring unnamed retrieval, one
      active problem per turn, with the outcomes as session evidence;
      pinned by `src/lesson.rs` tests.
      *Landed: the practitioner role's seed, the `PracticeConversation`
      seam with `practice` (one problem posed, answered, and judged
      before the next; a setting that covers the proposition's claim is
      refused by name, because a problem that names the answer is a
      recall test), and `record_practice` landing one `practice` fact
      per problem — the setting, the answer, and whether it held; the
      exercises ride the chain's field.*
- [ ] **C7 — the face surface.** The Learning rail section and pane
      (fed by the mockup) show the active lesson, the graph, the
      practice entry, and the history; pinned by a
      `tests/face/harness.mjs` case.
- [x] **C8 — the feeds.** The demand map's classes include the
      learning rows' confusions, and the capability gaps land through
      the improve store and the drafter; pinned by tests over a
      fixture lesson.
      *Landed: the `learn_check` row (one per taught step — the
      proposition, the learner's own confusion, whether it held) and
      the demand map's `learn` family (the class is the proposition, a
      row is bad when the learner named a confusion); the clustering
      beat files the repeating confusion into the improve store and the
      proposal's justification names the family; the drafter files a
      skill draft over a lesson session whose procedure repeated.*
- [x] **C9 — the evidence discipline.** The probe records carry the
      thinking and the confidence; the sessions and the graph stay
      distinct (the graph never claims mastery); pinned by the record
      shapes' tests.
      *Landed: the graph's record carries the claim, the premises, and
      the provenance — no confidence, mastery, score, or held line
      (pinned on the file's own text); the session's records carry the
      held and the answer, and the probe's carry the thinking and the
      confidence.*
- [x] **C10 — the gate.** fmt, clippy, release build, test — exit
      codes read directly, the whole suite.
      *Landed: `cargo fmt --all -- --check` exit 0, `cargo clippy
      --all-targets -- -D warnings` exit 0, `cargo build --release`
      exit 0, `cargo test` green — 1,999 lib tests and every
      integration suite, the two lanes tests green under
      `BROWSER_CHROME` (the environmental input the browser lane needs);
      the face harness 998 checks across 117 cases; `plan_check.py`
      exit 0. The live CLI proof: `fleet sessions --kind teaching` lists
      the teaching session, `--kind practice` none, the default both.*
- [x] **C11 — the docs.** `docs/features/learning.md` (new), the
      surfaces doc's rail inventory, and the memory doc's proposition
      type and premises field.
      *Landed 801d9da: the feature doc (the theory, the four sections,
      the loop, the records, the evidence discipline, the feeds), the
      surfaces doc's learning group and section, the memory doc's
      `premises` field and `proposition` type, and the README's
      pointer.*

## Files to be touched

**Core:**
- `src/session.rs` — the kind on the `Start` record.
- `src/lesson.rs` — new: the probe loop, the plan seam, the teaching
  flow, the practice sessions, the records.
- `src/graph.rs` — the lesson plan's DAG and its critical path (the
  teaching order).
- `src/calib.rs` — the `learn_probe` row.
- `src/memory.rs` — the `premises` field and the proposition type.
- `src/lesson.rs` — the lesson roles' overlay admission (the
  prober/teacher/practitioner ride the run's overlay seam, which the
  agent's overlay field carries; they are overlays, not delegation
  specializations).

**Face:**
- `src/gui/assets/app.js`, `src/gui/bridge.rs` — the Learning
  section and pane.
- `tests/face/harness.mjs` — the pane's pin.

**Documentation:**
- `docs/features/learning.md` — new.
- `docs/architecture/surfaces.md` — the rail inventory.
- `docs/features/memory.md` — the proposition type.

## Out of scope

- **The host-specific machinery.** The Electron shell, the provider
  and subscription connections, the dictation, the in-app updater —
  fleet's equivalents exist (the Tauri face, the provider connect
  flow).
- **The background research workers.** The research/PDF/demo/visual
  dispatch maps to fleet's lanes and subagents; a later slice if a
  lesson needs them.

## Verification

```sh
# The mockup: measured, read
cargo test -p fleet --lib lesson   # the loop's pins
cargo test -p fleet --lib graph    # the plan's pins
cargo test -p fleet --lib memory   # the records' pins
node tests/face/harness.mjs        # the face's regression
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test   # C10
```

The live proof: the owner's first lesson runs end to end — the probe
brackets, the plan draws, the teaching moves one checked step at a
time, the graph accumulates, and the friction's proposals land in the
stores.

## Linked artifacts

- `docs/features/learning.md` — the Learning surface.
- `docs/architecture/surfaces.md` — the rail inventory.
- `docs/features/memory.md` — the proposition type and premises.
- `docs/architecture/deep-dive-2026-11-08-learnharness.md` — the
  design record this plan implements.

## Risks

- **The probe's precision is the plan's ceiling.** The records carry
  the confidence and the thinking so the bracket is auditable.
- **The teaching's single-step discipline.** One checked step at a
  time, the repairs first; a multi-step lecture is the failure the
  theory names.
- **The extraction's quality.** The graph is only as good as the
  extract-audit-merge; the gate and the dream pass's reading bound it.
- **The learner's thinking capture.** The fields are earned in the
  conversation, not demanded.

## Notes

The phases land as their own commits and each is exercised live
before the next — the fast loop: the host and the measure, then the
plan, the teaching, the graph, the practice, the face, the feeds. The
owner's first lesson is the proof; a failure is fixed in the open, not
deferred to a later plan.
