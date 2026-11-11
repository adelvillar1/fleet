# Selection — the decision-head seam

The grounded-selection plan's feature doc: the seam's contract, the
measured baseline, every question shape tried (wins and failures), and
what each taught about asking. Status mirrors
`docs/plans/2026-11-10-grounded-selection.md`.

## Baseline (C0, measured 2026-11-10)

```sh
cargo run --bin fleet -- tools audit --all
```

Over the whole archive — 764 session logs played, 0 skipped:

| Number | Value | Denominator |
|---|---|---|
| Tool calls | 26,556 | — |
| Failures (error/denied results) | 915 (3.4%) | calls |
| **Wrong-tool rate** | **0.9%** (235) | calls — refused resolutions |
| **Malformed-argument rate** | **0.1%** (28) | calls — `error[validation]` |
| **Repair-turn rate** (recovery) | **54.2%** (496) | failures |
| **Mean declaration tokens/request** | **8,028 est.** (chars/4) | 9,321 requests |
| Mean declared tools/request | 27.3 | same |
| Mean prompt tokens/request | 70,167 | 9,413 requests |
| Resolutions | exact 20,584 · refused 235 · retrieved 2 | recorded |

The declaration tax is ~11% of every request. `retrieved` resolutions
are 2 of 20,821 — the deferred-tools machinery almost never runs.
Re-run after each phase; the criteria compare against this table.

## Head-to-head (C1, measured 2026-11-10) — the kill gate

```sh
python3 scripts/selection-headtohead.py --sample 120 --providers glide,drex,liquid
```

120 recorded selection turns, each asked of every head over that
session's own declared catalog. Ground truth: the recorded choice when
the call resolved `exact`/`alias` and did not fail. The head sees the
intent alone — deliberately harder than the model's context-full task.

| head | answered | top-1 | top-3 hit | accuracy | margin AUC | errors |
|---|---|---|---|---|---|---|
| drex | 120 | 68 | 75.8% | 56.7% | 0.756 | 0 |
| glide | 119 | 61 | 76.5% | 51.3% | 0.787 | 1 |
| liquid | 119 | 34 | 55.5% | 28.6% | 0.568 | 1 |

**The kill gate fired.** No head approaches the model's own 99.1%
name-accuracy; C2's injection would leave one turn in four without the
wanted tool in its top-3. **Phases 1-3 (C2-C9) stopped.** What the data
keeps: the heads are calibrated (glide margin AUC 0.787), liquid d1 is
the wrong head for selection (28.6%, AUC 0.568), and Phase 5's seams
are different tasks with their own gates.

## The verdict ledger: fix, improve, reject

**Final triage (2026-11-10 evening, the owner's direction):**

- **KILLED** (proved no value, recorded on the plan): C2, C3, C5, C6,
  C7, C8-as-designed — selection replacement and its dependents — plus
  **the repair leg's repair authorship** (LFM3 recovery 29.3% vs the
  loop's 54.2%; the leg demoted to `experiments.repair_leg` opt-in;
  its DETECTION value — 0.983 AUC — stands, shipped).
- **KEPT** (proved value, shipped in 0.6.25/0.6.26): the generation
  seam (C10/C11), the destructive head + experiment gate (C12 +
  experimentation-over-refusal), vision QC (C17), the form-filler's
  remember/improve origins (C19, opt-in repair now), the graded
  judgment stream (C9's remnant), the archive audit (C0's command).
- **FURTHER EVALUATION** (inconclusive, named backlog on the plan):
  C13-mention/silent, C14, C15's per-item score variant, C16 — all
  gated on small labeled fixtures — plus the live-grounding passes.

**Earlier rejections (this page, above):** selection replacement's
eight shapes, score fan-out at catalog size, LFM3 as selector,
cross-vendor score transplant, hard narrowing, the canonical framing
for selection, d1 for selection, the declaration-tax "fix".

**Original triage (2026-11-10 afternoon), superseded by the final:**


**FIX** — broken, with an evidence-backed repair in reach:

- ~~**The declaration tax**~~ — **REJECTED as a fix, corrected
  2026-11-10**: the bound's own history (agent.rs:200-206) pins the
  native surface at 10,711 tokens against the 10,800 bound — six
  crossings caught by `the_native_surface_fits_its_own_bounds`. The
  8,028 mean is the complete surface fitting its bound, not waste; the
  deferral machinery engages on overflow and `tool_search` covers the
  deferred (2 fires in 20,821 = the model rarely needs them — the
  machinery working as designed). Tightening the cap would break the
  native surface's fit and raise wrong-tool. The tax is the honest cost
  of a complete surface.
- **KI-515** — the three stale blend1 corpus tests (unpack 4 of a
  5-value contract): update them; mechanical.
- **KI-516** — the layerfs coherence test's schedule flake: its own
  stub log, or the AMBIENT_LOCK held across the assertions.
- **The silent doc debt** — TECHNICAL-DOCUMENTATION and
  FUNCTIONAL-SPECIFICATIONS still owe the seams' sections (the plan's
  linked artifacts); the recap step closes it.
- ~~The plan-gate's dead chain~~ — **downgraded 2026-11-10**: the
  interim pin `plan_gate = "drex,jev"` answers through drex (verified
  live: 4/4 coverage, PASS). Only the jev tail is dead; buying TypeSafe
  credits is optional, nothing is blocked.

**IMPROVE** — working, with a measured next step:

- **The composed selection family** (weed-then-verify 54.6%, the
  conditioned pipeline 55.5%/79.0%) — the verifier measurement on the
  corrupted-call corpus decides whether it earns the runtime build
  (C4's re-open). The last unmeasured shape.
- **The destructive head (C12)** — live; the vendor's gate-cookbook
  framing is the upgrade path (state.policy + state.goal beside the
  command), and its thresholds learn from the approval outcomes the
  stream already records.
- **The vision QC (C17) and the form-filler (C19)** — landed behind
  stub tests; one live pass each grounds them (a real gen1 image
  graded; a real repair rate on the corrupted corpus).
- **The end-of-run prose (C10)** — the cost table is estimated; one
  live fold + title + summary through the real lane measures it.
- **The C14-C16 fixtures** — the extraction is buildable now so the
  owner's labeling is copy-and-judge; each labeled fixture unlocks its
  seam's build-or-kill.

**REJECT** — abandoned on measurement, not opinion:

- **Selection replacement** (C2, C3, C7 and every question shape aimed
  at it) — eight shapes, three vendors, two framings; the task is
  under-determined from intent alone. The plan's own kill stands.
- **Score fan-out at catalog size** — the 27-head collapse is measured;
  per-item separate calls would be 27 requests per turn for a worse
  ranking than the weed's own.
- **LFM3 as selector or re-ranker** — 20-30% measured; its home is
  prose and form-fills (C10/C19, landed).
- **Cross-vendor score transplant** — identical to control on both
  mixes; cross-vendor ensembles need an arbitrating call nobody has
  justified yet.
- **Hard narrowing on invented taxonomies** — conditioning dominates it
  in every measured pair.
- **d1 for selection** — 28.6%; its measured strengths (vision_qc,
  needs_generation) are where it already serves.
- **C8 speculative pre-warm as designed** — it fed off head selection;
  only a verifier-shaped variant (pre-checking the model's own emitted
  call) survives, and only if the verifier lands.

## The verifier measurement (2026-11-10) — the re-open evidence

The corrupted-call corpus: 60 recorded successful calls, each corrupted
two ways (argument-level: dropped field, swapped values, mistyped
type; name-level: a plausible wrong tool), the original as ground truth
by construction. The noul question: "is this call correct for the
intent?" (`scripts/c-verifier-measure.py`):

| vendor | separation AUC | corrupted rejected at 0.5 | original accepted at 0.5 |
|---|---|---|---|
| **glide** | **0.983** | **60/60 (100%)** | 41.7% |
| drex | 0.883 | 58/60 (96.7%) | 45.0% |

**The verification signal is real and cross-vendor consistent**: the
heads separate originals from corruptions near-perfectly, and catch
every corrupted call at the 0.5 threshold. The asymmetry is the design
instruction: the heads are conservative acceptors (they reject 55-58%
of good calls too), so the verdict **composes with the model's own
choice rather than gating it** — the weed-then-verify lesson at the
emission. The runtime shape this earns: every emitted call verified;
noul-below-threshold triggers the repair leg (C19's form-filler and the
head-rename path already exist); the model's signal stands otherwise.
**C4's re-open is evidence-backed**: the verify+repair leg builds on
this measured basis.

## The failure review: every miss, and what it taught about asking

Ten shapes measured across three vendors on the same 120 turns. Each
failure, its score, and its lesson:

**1. A1/A2c — taxonomy narrowing** (45.0% / 42.5% top-1; failed
identically on drex at 44.9%). Asked "which class?" then "which tool in
class?". Failed because the taxonomy was invented and uncalibrated —
and hard narrowing makes a pass-1 miss unrecoverable. **Lesson:
condition, never hard-narrow; a narrowing error compounds,
conditioning keeps the option set alive.**

**2. B — noul fan-out over every tool** (49.2% top-1, top-3 collapsed
to 62.5%). Asked "does each tool serve the intent?" independently.
Failed because tool candidates are mutually competitive — choosing one
is not-choosing the others — and independent judgments cannot express
that. **Lesson: decompose only properties that are genuinely
independent.**

**3. Score fan-out** (1.7% top-1 — below random). Asked 27 ordinal
grades in one request. The diagnostic showed all 27 heads collapse to
score=5.0 with identical confidence — a wire artifact, not an answer.
**Lessons: diagnose below-random results before believing them; the
documented examples carry 2-3 heads per call, and 27 score heads sits
outside the practical envelope; the vendor's "score each item
separately" means separate calls.**

**4. LFM3-in-the-middle** (re-rank 30.3%, weed 20.2%). Asked the small
generative model to rank and to shortlist. Failed: its selection
judgment is weak — its own vendor caveat ("not recommended for agentic
coding") is real. **Lesson: role-assign by measured strength, not by
availability.**

**5. Cross-vendor verification** (identical to control on both mixes).
Glide's noul on drex's shortlist and the reverse. Failed: noul
calibrations do not transfer across vendors on this task. **Lesson:
compose within a vendor's calibration; cross-vendor ensembles need an
arbitrating call, not a transplanted score.**

**6. Canonical framing** (52.5%/54.2% top-1 — no gain; top-3 dropped
~12pp on both vendors). The vendor's own routing recipe applied:
described criteria on every label, an explicit `no_tool` escape route,
structured state, native margin. Failed to gain because tool options
are too semantically similar for descriptions to separate — the
descriptions diffuse the distribution
(`scripts/c-canonical-measure.py`). **Lesson: descriptions help when
the destinations are distinct; on similar options they add plausible
competitors.**

**7. The meta-failure — selection-from-intent itself.** Eight shapes,
three vendors, two framings; nothing approaches the model's own 99.1%.
The root cause is not any question's wording: **the recorded choice
depended on conversation state — tool results, prior calls, the
model's plan — that no intent-only question carries. The task was
under-determined from the start.** Lesson, and the review's first
rule: **before measuring shapes, test whether the answer exists in the
information the question carries.**

**What worked, and its shared shape:**

- weed-then-verify product (54.6% glide top-1) — comparative x
  independent signals multiplied
- the conditioned pipeline (55.5% / 79.0% — the best glide family) —
  soft conditioning + verification
- the destructive head (1/1 recall, 76/77 precision) — one independent
  property, one question

Every winner asks a question whose answer is IN the supplied
information, judges an independent property, and (when composed)
multiplies with a comparative signal from the same vendor. Every loser
violates one of those three. The runtime question that passes all
three — "is this emitted call right?" — is the verifier measurement
still unrun.

## The composite replays (2026-11-10)

The owner's stacked-calls question: break the selection into multiple
atomic sys1 calls. Shapes measured
(`scripts/c-composite-measure.py`, `scripts/c-atomic-measure.py`,
`scripts/c-score-measure.py`, `scripts/c-lfm3-measure.py`):

| shape | top-1 | top-3 |
|---|---|---|
| flat — drex (best single) | **56.7%** | 75.8% |
| flat — glide | 51.3% | 76.5% |
| weed-then-verify product — glide | 54.6% | 78.2% |
| conditioned pipeline — glide | 55.5% | **79.0%** |
| B noul fan-out — glide | 49.2% | 62.5% |
| A1 classify→decide — glide / drex | 45.0% / 44.9% | 45.8% |
| A2c top-2-class union — glide | 42.5% | 63.3% |
| L1 LFM3 re-rank — glide weed | 30.3% | 35.3% |
| L2 LFM3 schema-aware weed | 20.2% | 22.7% |
| score fan-out — glide (wire artifact) | 1.7% | 2.5% |

Cross-vendor verification (drex-weed × glide-verify and the reverse)
measured identical to control: the noul calibrations do not transfer.

## Weed-then-verify (the owner's shape, 2026-11-10)

One flat pass weeds to top-5; one batched noul request verifies each
survivor; the re-rank combines the signals
(`scripts/c-weed-measure.py`, weed×verify provider-choosable):

| variant | glide top-1 / top-3 | drex top-1 / top-3 |
|---|---|---|
| the weed's own pick | 52.1% / — | 53.0% / — |
| noul-only re-rank | 35.3% / **79.8%** | 51.3% / **79.5%** |
| **flat × noul product** | **54.6%** / 78.2% | 53.0% / **79.5%** |
| flat alone, same subsets | 52.1% / 78.2% | 53.0% / 76.9% |

The noul pass improves the top-3 ordering (+2.5pp) on both vendors even
where it does not improve top-1; multiplied with the flat probability —
never alone — it nudges glide's top-1. The two signals compose because
they measure different things: the flat pass is comparative, the noul
pass independent. Per-run gains are 2-3 turns (within noise
individually); the cross-vendor consistency is the evidence.

## The conditioned pipeline (the owner's second shape, 2026-11-10)

Shape-condition the question, keep the full catalog visible, verify the
winner (`scripts/c-conditioned-measure.py`): Q2 shape-conditioned
choice 49.6% / 78.2%; **P (Q2 × Q3 noul verification): 55.5% top-1 /
79.0% top-3 — the best glide family measured.** Three independently
designed compositions land in the same band (54.5-55.5% / 78-80%):
the composed family genuinely refines the flat pass, and its value is
the verification signal — exactly what the verifier measurement tests
at the emission.

## The destructive head (C12, measured 2026-11-10)

`scripts/c12-destructive-measure.py`: 78 real bash commands from the
archive — every substring-list flag (59) plus a stride sample of
unflagged ones — hand-labeled on one rule. One command destructive; the
rest temp-file, lock, and probe cleanup.

| classifier | destructive recall | safe precision | false flags |
|---|---|---|---|
| **GLiDE yes/no head** | **1/1** | **76/77** | **1** |
| fleet's `DESTRUCTIVE` list | 0/1 | 69/77 | 8 |

The head catches the tracked-file removal the list cannot see and
false-flags once where the list flags eight. Landed as the pre-pass in
`execute_calls`: the list stays the zero-spend fast path, an unlisted
bash command asks the head through an inline `task_spec`, and a `yes`
annotates the call so the ordinary gate asks. A dead lane changes
nothing.

## Head-to-head notes

The measurement reads the logs alone where it can; the replay harness
(`scripts/selection-headtohead.py`) is the shared turn loader for every
shape on this page — same 120 turns, same ground truth, deterministic
stride — so every scoreboard above compares like with like.
