# Slice 17 — the explanation beat: fleet explains its own verdicts (2026-10-14)

status: landed-pushed (2026-10-14; reviews closed, the fix round in 9caee0d, the recap in docs/recaps/2026-10-14-fleet-slice-17-explain.md)

## Goal

The owner mapped the same five-lane consumption onto dev-decisions, and
its new synthesis lane gives every verdict-emitting gate a second beat:
a synthesized, validated explanation of what the verdict means and what
the author should change, riding in the JSONL beside the verdict and
keyed by its input sha256 — under the discipline **blend1 proposes
prose, the gate disposes** (eval-only, batch-only, artifact-integrity
checked, recoverable text, `--dry-run` spends nothing). Fleet now owns
the lane (slice 16), so the beat ports natively: every fleet verdict
that today names its failure without its cause — the improve gate's
red, the eval corpus's failed scenario — gains the explanation, and
the improve loop gains the artifact-drift guard the same pattern
carries.

## The problem shape (PS-A, from slice 16's derivation)

"**Why did the gate fail?**" — the naive path is the operator reading
the failing step's output and reconstructing the cause by hand, every
time. A red `fleet improve apply` says *which* of fmt/clippy/build/test
failed; a regressing eval scenario names the check that did not hold;
neither says what the failure *means* or what to change. The
explanation is the composed cause, and it must never become a second
verdict — the gate's verdict stands exactly as recorded.

## Acceptance criteria

- **S1 — the surface.** `fleet explain` with a verdict selector —
  `--proposal <name>` (the improve gate's verdict) and `--eval
  <scenario>` (the corpus's) — plus `--dry-run`. It reads the verdict's
  row from the calibration stream, composes the ask (the verdict's
  evidence plus the judged artifact), calls the blend1 lane, and
  renders a card. An unknown selector is a named refusal.
- **S2 — the discipline (the port, verbatim in spirit).** The
  explanation never gates, blocks, or applies anything: the verdict
  stands exactly as recorded and the row carries an eval-only marker.
  Batch-only: no synchronous hook path references the explain module —
  pinned by a socket-guard test in dev-decisions' shape (the doctor's
  boot checks, the recall path, and the tool loop's synchronous paths
  are the guarded set). `--dry-run` spends nothing: the ask is not
  made and the card shows what would be asked. A named daily budget
  (`EXPLAIN_BUDGET_USD_DAY`, a constant with a config seam) caps the
  spend; exceeding it is a named refusal carrying the day's spend.
- **S3 — artifact integrity.** The verdict row carries the judged
  artifact's identity (`input_sha256`, the devdec join shape — the
  sha helper already exists at `src/calib.rs:454`; the propose row and
  the eval row gain the field). The explanation runs against the
  recorded artifact and a sha mismatch is a named refusal: explaining a
  drifted artifact is worse than not explaining.
- **S4 — recoverable text.** The explanation lands on disk at a
  documented path under the home (`explanations/<input_sha256>.md`,
  dev-decisions' shape). The row carries `explanation_sha256`,
  `explanation_chars`, the ask's route/validation telemetry, the cost,
  and the linked `input_sha256`; the session log and the calib row stay
  redacted — no prompt or answer text ever enters either.
- **S5 — the verdicts covered.** The improve gate's verdict (per
  proposal: the failing or refused step, the per-step evidence) and the
  eval corpus's failed scenario (the scenario's checks and the first
  that did not hold), both proven through the real binary against the
  scripted seam, and both verified to leave the verdict untouched.
- **S6 — the apply's drift guard (PS-C).** A proposal carries its
  targets' content shas at draft time; an apply whose target's sha
  changed since the draft is a named refusal beside the existing
  dirty-tree guard (KI-103), pinned by a regression test.
- **S7 — the UX at the house bar.** The card: the verdict's shape, the
  explanation's text, the route/validation telemetry, the cost —
  verified through the PTY capture plus a deterministic probe (the
  headless card and the TUI surface agree; nothing clipped at 80
  columns). Never a plain-text dump.
- **S8 — the lane fails open, named.** A lane that cannot answer
  (unconfigured, down, past its bound) names its reason and the verdict
  stands — never a silent absence, never a fabricated explanation. The
  ask's spend is bounded by the lane's own route (the direct path costs
  nothing; the gated synthesis measured at ~$0.0018 in the sibling).
- **S9 — the eval scenario.** A deterministic scenario grades the
  discipline: a failed verdict explained beside it, the verdict
  unchanged, the row redacted, the text recoverable by its sha.
- **S10 — the docs.** `docs/features/explain.md`: the pattern and its
  provenance (dev-decisions' synthesis lane, with the differences
  named), the discipline, the surfaces, the budget, the artifact rule,
  and PS-A with its combinations (PS-A + PS6: the checked claim gains
  its cause; PS-A + PS3: the drift's cause beside the drift).
- **S11 — the gate.** The full suite green, nothing deleted,
  warning-free; every error or warning encountered lands in the ledger
  with its evidence.

## Design decisions

1. **Native, not a dev-decisions runtime dependency.** The cross-repo
   value is the pattern; the machinery is fleet's own vendored lane.
   C5's self-contained rule stays intact — no CLI, no checkout, no
   network dependency beyond the lane the crate already carries.
2. **Eval-only by construction.** The explanation is a new calib op
   (`explain`) that no verdict reads. There is no code path by which an
   explanation could change a gate's outcome — the socket-guard test
   and the row's marker are the proof, not the intention.
3. **The budget is a named constant with a config seam.** The spend is
   real and per-ask; a daily cap with a named refusal is the
   difference between a feature and a money pit. The cap's value is a
   constant the owner's config can lower, never silently raise.
4. **The artifact identity is a precondition, so it is built first.**
   The verdict rows gain `input_sha256` (S3) before the beat attaches —
   an explanation without a keyed artifact is the drift hazard the
   sibling's rule exists to prevent.
5. **The first verdicts are the improve gate's and the corpus's** —
   the surfaces whose artifact identity already exists or is cheap.
   The doctor's beat follows when its artifact (the check report) is
   hashed; the boards' feed is a later surface.

## Out of scope

- **PS-B, the calibration loop closing** — dev-decisions' judgment lane
  judging fleet's route rows and the judgments riding back into the
  router's selection. The next slice's natural subject; named here so
  it is not silently dropped.
- PS-D (the semantic index over fleet's own stream) and PS-E (the
  media beat on fleet's surfaces).
- The doctor's explanation beat and the boards' explanation feed.

## Assumptions (listed, not asked)

- The blend1 lane's `ask` is the synthesis path (the router decides);
  the direct path — a quote from the verdict's own evidence — costs
  nothing and is the common case.
- The spend is the owner's; the budget caps it per day and the refusal
  names the spend.
- The calib vocabulary gains fleet's `explain` op; dev-decisions reads
  it as it reads fleet's other ops (the vocabulary's owner is told,
  not asked).
