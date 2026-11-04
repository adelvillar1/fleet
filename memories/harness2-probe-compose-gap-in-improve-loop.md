---
name: harness2-probe-compose-gap-in-improve-loop
description: "google-research/harness2 read 2026-10-19 — its contrastive Propose-Probe-Compose step is the measurement half fleet's improve loop lacks; the substrate (eval score + calib rows) already exists"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

`google-research/harness2` (arXiv:2610.14224, Python, Apache-2.0, "Recursive Agent Harnessing for an Open World"): with model weights fixed, recursively improve the HARNESS. Two coupled levels — task-level **Propose-Probe-Compose** (propose candidate edits, probe contrastively by executing the same task under base and candidate harness, compose the adopted one) and a harnessing-level "improvement harness": a per-domain library of adopted harness trees + `EXPERIENCE.md` carrying cue/lesson/evidence/confidence (stored only when all three evidence fields are non-empty), retrieval by construction (domain directory, INDEX.md rebuilt from disk). Eight editable components declared as data (`SurfaceSpec`: always-on/skills/subagents/scripts/plugins/hooks + caps + per-surface validators); **omitted-is-inherited** edit contract (a proposal emits only what it changes); **a judge failure is None, never a zero**; diversity = pairwise line-Jaccard collapse detector; validation errors are fed back verbatim as retry feedback (one retry, then inherit the base).

Fleet's `improve.rs` is a proposal-gate-apply loop with **no probe**: the eval corpus runs only for source edits, as a binary green/red regression gate, and `src/eval.rs:903`'s grade_output mislabels a nonzero exit with no `regressions:` line as "regressed" — the exact misattribution eval.rs's own docstring warns about. Nothing records or consults the outcome of an applied change. Fleet is ahead on: the local gate (fmt/clippy/build/test), git pre-image + byte-for-byte undo, destructive changes waiting for owner approval.

The one high-value port: **adopt on a measured before/after contrast instead of on no-breakage alone** — `EvalReport::score`, the `eval_corpus` calib rows and `fleet eval --last` already record the history the loop never reads — then land an outcome row and a lesson. Planned 2026-10-19 as `docs/plans/2026-10-19-adopt-on-contrast.md` (status **completed** the same day, uncommitted): P1 the probe's three outcomes (`ProbeOutcome::{Green,Regressed}`, `Err` = could-not-be-measured, `eval-score:` footer line, unmeasured ≠ regressed), P2 the baseline via `eval::latest_row` + the contrast in the verdict (corpus instruments source targets only — the scope honesty is written into the verdict), P3 the evidence-required lesson store (`<home>/lessons/`, cue/lesson/evidence/confidence, handed to the model in the `improve` tool's per-run description and named in the filing confirmation), P4 the `last-refusal` ride-back in the frontmatter + card, P5 the skills collapse detector (`fleet skills --duplicates`, line-Jaccard 0.60, report-only). Gate green: fmt/clippy/build/test, lib 1187; eval corpus 21 scenarios (new `improve-lesson-store` family); live `--duplicates` over 442 skills reports no pairs at 0.60. The SurfaceSpec refactor remains out of scope as its own later plan.
