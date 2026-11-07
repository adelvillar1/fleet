---
status: active
created: 2026-11-07
updated: 2026-11-07
slug: self-improvement
---

# Plan: the self-improvement program — the demand map and seven instruments

## Context

The 2026-11-07 deep dive
(`docs/architecture/deep-dive-2026-11-07-self-improvement.md`) read
DoubleAI's "Can LLMs redesign themselves as they train?" against
fleet's self-improvement machinery — the improve proposal loop, the
eval corpus, the judge and delegate beats, the calibration stream, and
the dream pass landed the same day. Eight improvement areas came out,
and this plan builds them as a sequenced program: each phase lands
independently, and the demand map (C0) is the foundation the others
read from, the article's own argument for the phantom gradient.

Decision-model check: every phase below judges **per item against
evidence** — a failure class from the rows, a threshold from the
graded outcomes, a probe verdict through sys1, a race against a twin
and a sham. The house's judgment lane is the judge each phase names;
no phase invents a decision seam.

## Direction lock

- **The demand map first.** Every other phase reads the per-class
  demand the map aggregates from the calib stream; nothing downstream
  grades a class the map cannot name.
- **Fleet's interventions stay owner-gated.** The article's agent
  acts inside the loop; fleet's ruling is the opposite. Every phase
  produces proposals, measured gates, or named actions — never silent
  self-application. The one exception is the re-run seam (C2), which
  re-dispatches a delegated child the owner already approved; the
  re-dispatch is logged and bounded like the first.
- **Measured before acted.** The article's zero-init gate and Qwen's
  analysis step agree: a candidate proves itself on held-out evidence
  before it acts. Each phase carries its measured bar.
- **The beats' raw material is the stream.** No phase invents a new
  store; the calib rows, the eval corpus, and the proposal store are
  the substrates, and every phase's output lands in one of them.

## Approach

The build rides the stream's own rows and the existing beats: the
demand map is a new reader over `calib::read_days_recent`; the
clustering and analysis phases are new modules beside the beats (the
socket-guard shape: lane-owning wrappers in their own modules, never
named from the scanned synchronous paths); the race extends the eval
corpus's runner; the timing phase extends the beats' scope; the
capability view reads the applied proposals.

Phases, in dependency order:

0. **The demand map (C0).** `src/demand.rs`: a reader that aggregates
   the calib stream's rows into per-class demand — decision classes
   (workload × provider × verdict), eval scenarios, delegation
   outcomes, validate verdicts — each with its recent rate and its
   trend (improving vs flat). `fleet demand` prints it.
1. **Clustering and shape proposals (C1).** Group a period's failures
   by class, surface each cluster's examples, and file a shape
   proposal for a cluster that repeats — through the improve store,
   owner-applied.
2. **The self-probe acting per decision (C2).** A delegation whose
   outcome probe confidently says "wrong" is re-run: the probe is
   sys1 over the child's log against its brief, the action is the
   existing re-dispatch, logged and bounded.
3. **Twin and sham races (C3).** `fleet improve race <proposal>`:
   the corpus on the applied tree, on the twin (change reverted), and
   on the sham (the diff scrambled); the proposal's effect must beat
   both.
4. **Analysis before policy (C4).** `fleet improve analyze`: the
   threshold question answered from the stream — which classes fail,
   how consistently, what threshold separates — before a proposal's
   acceptance criteria are written.
5. **Timing (C5).** The judge and delegate beats accept a session or
   run scope, not only a day's, so an intervention can land while the
   context that produced it is live.
6. **Jaggedness and the gaps (C6).** `fleet eval --gaps` names the
   classes the stream shows but the corpus does not cover; a gap
   scenario is promoted into the corpus from the rows' examples.
7. **The capability library (C7).** The applied proposals rendered as
   capabilities — the check, pattern, or threshold each established —
   feeding the skill drafter's feedstock.
8. **The gate (C8).** `cargo fmt --all --check`, `cargo clippy
   --all-targets -- -D warnings`, `cargo build --release`,
   `cargo test` — each exit code read directly, the whole suite, run
   before every push that carries a phase's code.
9. **The docs (C9).** `docs/features/self-maintenance.md` and
   `docs/features/calibration.md` gain the demand map, the race, the
   analysis, the gaps, and the capability view; the tech doc's Module
   reference names `src/demand.rs`.

## Use cases

- [ ] A: A decision class fails flat across runs and nothing names it
  — the demand map shows the class, its rate, and its trend, and the
  clustering phase files the shape proposal.
- [ ] B: A proposal's effect is real or placebo — the race runs the
  twin and the sham, and only a proposal that beats both is applied.
- [ ] C: A delegation's outcome looks wrong but nobody re-checks it —
  the probe grades it, and a confident wrong triggers the re-run.
- [ ] D: A proposal's thresholds are guessed — the analysis answers
  them from the graded rows first.

## UX routes

No new surface beyond CLI verbs and one face section: `fleet demand`
cards; the demand map joins the face's calibration surface as a
section; the race, analyze, and gaps verbs print cards; the capability
view rides the existing `fleet improve` surface.

## Acceptance criteria

- [ ] **C0 — the demand map answers per class.** `fleet demand` in
      `src/main.rs` prints the per-class demand aggregated from the
      calib stream by `src/demand.rs` — each class with its recent
      rate and its trend — and one `demand` calib row records the
      view; pinned by `src/demand.rs` tests over a fixture stream.
- [ ] **C1 — repeated failures become shape proposals.** The
      clustering beat in `src/demand.rs` groups a period's failures by
      class, names each cluster's examples, and files a shape proposal
      into `<FLEET_HOME>/proposals/` for a cluster at or over a named
      repetition threshold; pinned by tests over a fixture stream.
- [ ] **C2 — a confidently wrong delegation is re-run.** The delegate
      outcome probe through the lane judges the child's log against
      its brief; a confident wrong re-dispatches the child through the
      existing delegation seam, logged as a `delegate` row with the
      probe's verdict; pinned by tests with the mock lanes.
- [ ] **C3 — the race separates real from placebo.** `fleet improve
      race <proposal>` in `src/improve.rs` runs the eval corpus on the
      applied tree, the twin (reverted), and the sham (scrambled
      diff), and reports the three scores; a proposal below either
      baseline is refused by name; pinned by tests over fixture
      proposals.
- [ ] **C4 — the thresholds are measured, not guessed.**
      `fleet improve analyze` in `src/improve.rs` answers the
      separation question from the stream — which classes fail, how
      consistently, the threshold that separates — and the `improve`
      tool's declaration in `src/tools.rs` names it as the first step;
      pinned by tests over a fixture stream.
- [ ] **C5 — the beats scope to a session.** The judge and delegate
      beats in `src/judge.rs` and `src/delegate.rs` accept a session or
      run scope beside the day's, and grade the session's rows with
      the same verdict vocabulary; pinned by tests.
- [ ] **C6 — the gaps are named and promotable.** `fleet eval --gaps`
      in `src/eval.rs` reports the classes the stream shows but the
      corpus does not cover, and a reported gap promotes into a corpus
      scenario from the rows' examples; pinned by tests.
- [ ] **C7 — the capabilities are readable.** The applied proposals
      render as capabilities — each with its target and the check,
      pattern, or threshold it established — through `fleet improve`
      in `src/improve.rs`, and the skill drafter's feedstock in
      `src/skills.rs` names them; pinned by tests.
- [ ] **C8 — the gate.** fmt, clippy, release build, test — exit
      codes read directly, the whole suite.
- [ ] **C9 — the docs.** `docs/features/self-maintenance.md` and the
      calibration doc gain the demand map, the race, the analysis, the
      gaps, and the capability view; the tech doc's module reference
      names `src/demand.rs`.

## Files to be touched

**Core:**
- `src/demand.rs` — new: the demand map reader, the clustering beat,
  the shape-proposal filing.
- `src/improve.rs` — the race, the analysis, the capability view.
- `src/eval.rs` — the gap report and the promotion seam.
- `src/judge.rs`, `src/delegate.rs` — the session scope.
- `src/cli.rs`, `src/main.rs` — the verbs and their wiring.
- `src/tools.rs` — the improve declaration's analysis step.
- `src/calib.rs` — the `demand` row.
- `src/gui/bridge.rs`, `src/gui/assets/app.js` — the demand section.

**Tests:**
- `src/demand.rs`, `src/improve.rs`, `src/eval.rs`, `src/judge.rs`,
  `src/delegate.rs` — the pins.

**Documentation (see Linked artifacts below):**
- `docs/features/self-maintenance.md` — update
- `docs/features/calibration.md` — update
- `TECHNICAL-DOCUMENTATION.md` — Module reference

## Out of scope

- **Autonomous application.** Every phase's output is a proposal, a
  measured gate, or a logged re-dispatch; nothing applies itself.
- **The article's continuous gate.** Fleet's discrete gate is the
  ruling; the phases borrow the measured-set pattern, not the
  mechanism.
- **A new store.** The calib stream, the eval corpus, and the
  proposal store are the only substrates.
- **Re-grading the whole corpus per race.** The race runs the
  scenario family the proposal touches, not the corpus whole.

## Verification

```sh
python3 scripts/ki_ledger.py docs/known-issues.md
cargo test -p fleet --lib demand   # C0–C1 pins
cargo test -p fleet --lib improve  # C3, C4, C7 pins
cargo test -p fleet --lib eval     # C6 pins
cargo test -p fleet --lib judge    # C5 pins
cargo test -p fleet --lib delegate # C2, C5 pins
node tests/face/harness.mjs        # the demand section
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test   # C8, codes read
```

## Linked artifacts

- `docs/features/self-maintenance.md` — the demand map, the race,
  the analysis, the gaps, the capability view.
- `docs/features/calibration.md` — the `demand` row and the beats'
  session scope.
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" —
  `src/demand.rs`.
- `docs/architecture/deep-dive-2026-11-07-self-improvement.md` — the
  design record this plan implements.

## Risks

- **The socket guards.** The clustering and analysis phases judge
  through sys1; their lane-owning wrappers live in `src/demand.rs`
  and a sibling, never named from the scanned synchronous paths —
  the `src/dream.rs` precedent, and the preflight names the guard.
- **The race's cost.** Three corpus runs per race; the family scope
  bounds it, and the route cache's decision-keys gate bounds the
  lane asks.
- **The demand map's honesty.** A class with few rows must read as
  thin, not as zero — the trend carries the row count.
- **The re-run seam's bounds.** The C2 re-dispatch re-runs a child the
  owner approved once; the re-dispatch is bounded by the same
  `max-turns` and logged as its own row.

## Notes

The program's phasing is the dependency order: C0 lands first and
alone (it is the others' reader), C1 and C4 follow (both read the
map), then C2 and C3 (the probe and the race), then C6, C5, and C7.
Each phase lands as its own verified increment; the gate runs before
each push. The article's argument for the phantom gradient — every
other instrument reads the demand map — is the plan's own sequencing.
