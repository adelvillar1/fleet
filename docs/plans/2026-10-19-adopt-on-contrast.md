---
status: completed
created: 2026-10-19
updated: 2026-10-19
slug: adopt-on-contrast
---

# The improve loop gains a measurement step: adopt on contrast

## Context

`src/improve.rs` is a proposal-gate-apply loop: the harness files a
proposal against its own skills, memory, config, or source, and the
owner's `fleet improve apply` runs it through the local gate (`cargo
fmt --check`, `clippy -D warnings`, `build --release`, `test`) plus —
for source edits only — the eval corpus as a binary regression check.
Read beside `google-research/harness2` (arXiv:2610.14224, "Recursive
Agent Harnessing for an Open World"), three gaps name themselves:

1. **The gate proves "nothing broke", never "anything improved."**
   Harness²'s task-level recursion is a Propose–Probe–Compose step:
   propose a candidate harness, probe it by executing the same task
   under base and candidate, compose the adopted one from the execution
   contrast — with no ground-truth verifier anywhere. Fleet's loop has
   the propose and the veto and no probe. A green-but-inert change
   applies exactly like a change that moves the corpus; there is no
   adoption decision, only a refusal condition.

2. **The measurement substrate already exists and the loop never reads
   it.** `EvalReport::score` grades the twenty-scenario corpus (eleven
   families, deterministic, mock-driven), `fleet eval` lands one
   `eval_corpus` row per run in the calibration stream carrying
   passed/failed/score, and `eval::last_row` reads the most recent one
   back. The contrast signal is recorded on every run and ignored at
   the one decision point that needs it.

3. **Nothing learns.** After an apply, the `propose` row carries the
   verdict and the gate's per-step evidence, and that is the end of the
   story. No outcome is recorded later, no lesson is extracted, and the
   next proposal is filed with no knowledge of what past changes did.
   Harness²'s harnessing-level recursion is a per-domain library of
   adopted harnesses plus evidence-required lessons (cue, lesson,
   evidence, confidence) consulted before the next task.

A fourth, smaller defect blocks the first three and must land first:
`eval::grade_output` (src/eval.rs:903) reports a nonzero exit with no
`regressions:` line as "the eval corpus regressed" — so a crash, a
missing binary, or a timed-out probe is indistinguishable from a
measured regression. The eval module's own docstring already names this
misattribution (a driver panic's exit 101 read as a "regressed"
corpus, KI-027's spawn/port pattern promoted into the gate). Harness²'s
rule is the correction: **a judge failure is None, never a zero** — an
unmeasured run is excluded from the comparison, not scored as a bad
one.

## Approach

Five increments in dependency order. P1 first because a contrast is
only meaningful once "unmeasured" is distinguishable from "worse".

**P1 — the probe's three outcomes.** `grade_output` returns
`Ok(Green)` / `Ok(Regressed(names))` / `Err(could-not-run reason)`, and
the `EvalProbe` carries the run's score so the contrast never re-runs
the corpus. The gate keeps failing closed on `Err` — a source self-edit
that cannot be measured is not applied — but the gate step's detail and
the landed row say "could not run", never "regressed".

**P2 — the baseline and the contrast.** `improve::apply` reads the most
recent `eval_corpus` row (`eval::last_row`) as the baseline before the
change lands, and adopts on the measured contrast:

- after > baseline — applied, the summary naming both scores
  (`adopted: corpus 0.90 → 1.00`);
- after == baseline and green — applied, labeled plainly as no measured
  gain, because the corpus measures harness *machinery* and most
  skill/memory/config edits do not move it;
- after < baseline or red — refused by the gate exactly as today;
- unmeasured — the gate fails closed, named as unmeasured.

The scope honesty is written into the verdict text rather than implied
away: for skill, memory, and config targets the corpus is **not** the
instrument (a superseded memory fact does not change a scripted
scenario's outcome), and those targets keep adopting on the green gate
with the reason named — the plan says so instead of implying a
measurement that cannot exist. Source targets adopt on the contrast.
With no baseline row available (a machine whose stream has never run
the corpus), the contrast is "no baseline" and the green gate adopts
with that named.

**P3 — the lesson store.** A sibling of the proposal store,
`<FLEET_HOME>/lessons/`, one markdown file per applied change: the
proposal's name and target, the target artifact's sha, the measured
contrast (or the named unmoved-by-construction reason), and the
transferable lesson in the harness² vocabulary — cue, lesson, evidence,
confidence — under fleet's provenance discipline: a lesson with an
empty evidence field is refused, exactly as an inferred memory fact
needs its recorded justification before it supersedes. `fleet improve
lessons` renders the store, and the `improve` tool's filing path is
handed the recent lessons for the target kind before it files — the
read-before-propose half of Harness²'s harnessing-level recursion, in
fleet's dialect: the loop's own history guides its next proposal. The
store starts inside `improve.rs` beside the proposal store it mirrors
and is extracted to its own module only when it earns one, the crate's
own pattern.

**P4 — the refusal rides back.** The apply's refusal reason is stamped
into the proposal file (a `last-refusal` field) and shown by
the bare `fleet improve` listing, so the next filing attempt is informed by the
named failure instead of rediscovering it. This is the cheap,
owner-gated form of their "YOUR PREVIOUS ATTEMPT WAS REJECTED" retry
banner: no model retry loop, the human (or the filing model) re-files
with the reason in hand.

**P5 — the collapse detector.** A deterministic pairwise line-Jaccard
over the skills library's `SKILL.md` bodies, reported by a named
surface (`fleet skills --duplicates`, threshold a named constant), so
the library's drift toward sameness — 441 skills today — is visible
before it becomes a loader problem. Report-only: nothing is deleted,
the way Harness²'s `diversity` is a visibility instrument and not a
pruner.

## Acceptance Criteria

- **C1 — three probe outcomes, each named.** `grade_output`'s unit
  tests drive all three shapes: green (a passing child),
  `Regressed` (a red child whose `regressions:` line names the
  scenarios), and `Err` (an unresolved executable, a spawn failure, a
  timed-out child, and a nonzero exit with no `regressions:` line —
  today's mislabeled crash). The gate step's detail differs per shape;
  the existing fail-closed behavior is unchanged and pinned.
- **C2 — the contrast in the verdict.** Through the stand-in gate seam
  `improve.rs`'s tests already use: a source proposal whose probe score
  beats the baseline applies with both scores named; an inert one
  applies labeled no-measured-gain; a regressing one is refused with
  the scenario names; an unmeasured probe is refused named as
  could-not-run; a missing baseline adopts with that named. The
  `propose` row's summary carries the contrast.
- **C3 — the lesson store, evidence-required.** Applying a change
  writes exactly one lesson file; a lesson whose evidence field is
  empty is refused by name and writes nothing; `fleet improve lessons`
  renders the store (name, target, contrast, cue); the `improve` tool's
  filing path consults the recent lessons for the target kind; one
  eval-corpus scenario grades the write, the refusal, and the render.
- **C4 — the refusal rides back.** A refused proposal's file carries
  `last-refusal` naming the gate's failure, and the bare `fleet improve` listing
  shows it; a subsequent successful apply clears it.
- **C5 — the collapse detector.** A deterministic test over a fixture
  library with a known near-duplicate pair reports the pair and its
  similarity; a library of distinct bodies reports none; the surface is
  report-only and deletes nothing.
- **C6 — the docs.** `docs/features/self-maintenance.md` carries the
  loop end to end: the three probe outcomes, the contrast rule with
  its scope honesty (which targets the corpus instruments and which it
  does not), the lesson store and its evidence rule, and the refusal
  ride-back.
- **C7 — the gate.** The full suite green, warning-free, nothing
  deleted. This plan edits the gate's own module, so the gate is the
  acceptance; the new eval-corpus scenario (C3) is part of the corpus
  the gate runs.

## Files to be touched

- `src/eval.rs` — the probe's outcome type and score, `grade_output`'s
  three-way split, the corpus scenario for C3.
- `src/improve.rs` — baseline read, the contrast verdict, lesson
  writing, the `last-refusal` stamp.
- `src/calib.rs` — read-path reuse (`read_op_day`, `last_row`); no new
  row op unless C3's lesson needs one.
- `src/cli.rs` — the `fleet improve lessons` subcommand, the
  `last-refusal` display in `list`, help text.
- `src/skills.rs` — the collapse detector (or `src/doctor.rs` if it
  belongs in the boot checks — decided at implementation, named here).
- `src/tools.rs` — the `improve` tool's spec text (the lessons it is
  handed).
- `docs/features/self-maintenance.md` — the loop as it now stands.

## Out of scope

- **The SurfaceSpec refactor** — declaring the editable surface as data
  (paths, caps, per-surface validators) with omitted-is-inherited
  proposals. It is the right next step, but it reshapes improve's
  target machinery and deserves its own plan.
- **Model-driven proposers** — Harness²'s `claude_cli`/OpenCode
  channels run with `--dangerously-skip-permissions` and unrestricted
  network; that contradicts the owner-gated apply ruling. The port is
  the discipline, never the executor.
- **Recursion budgets and parallel candidate pools** — their 4k-rollout
  machinery is benchmark-scale; fleet's loop is dogfooding-scale.
- **A second measurement instrument for skill and memory targets** —
  named, not built: the recall replay measured in the 2026-10-18 dive
  is the candidate instrument for memory targets, and a later plan.
- **Judging answer quality** — the explanation beat's subject beside
  the corpus's grades, unchanged here.

## Verification

- The full gate — `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo test`
  — read directly from each step's status, never through a pipe. This
  plan changes the gate's own module; the gate green is the acceptance.
- New unit tests: `eval.rs` (the three probe outcomes), `improve.rs`
  (the contrast's five branches through the stand-in seam), the lesson
  store (write, evidence-refusal, render), `skills.rs` (the collapse
  detector over a fixture).
- One new eval-corpus scenario: the contrast adopt end to end.
- Hand-verified in the landing turn: `fleet eval --last` reads the new
  row; `fleet improve lessons` renders; the bare `fleet improve` listing shows a
  `last-refusal`.

## Linked artifacts

- `google-research/harness2` — arXiv:2610.14224; the Propose–Probe–
  Compose read, a reference and never a dependency (hard rule 6).
- The 2026-10-19 analysis in project memory:
  `harness2-probe-compose-gap-in-improve-loop`.
- `docs/plans/2026-10-18-memory-and-cache-fixes.md` — the recall
  measurement, the candidate instrument for memory targets.
- `docs/features/self-maintenance.md` — the loop as it stands today.
