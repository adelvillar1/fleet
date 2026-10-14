# Slice 18 — the calibration loop closes: fleet's routes are judged (2026-10-14)

status: planned

## Goal

PS-B, named in slice 16's derivation and slice 17's out-of-scope: the
calibration stream stops being a record and becomes a signal. Fleet's
router already records every decision as a `route` row — the candidate
that served, how it was chosen, the workload, the attempts and their
outcomes — and nothing reads them back. dev-decisions' judgment lane
now judges its own decisions the same way, and its calibration surface
turns the graded rows into per-head curves and floor-fit readiness.
This slice ports that loop natively: fleet's route rows are judged
against their outcomes, the judgments ride back as rows, a calibration
surface shows the per-provider quality, and the failover's preference
consults it — conservatively, behind the sibling's fit requirement,
with the owner's declared chain untouched.

## The problem shape (PS-B)

"**Was that decision right?**" — the naive path is the operator
noticing, weeks later, that one provider has been quietly wrong for
this workload. Routing is self-grading: the outcome evidence is
already in the row — a failed attempt, a failover, a cheap successful
turn — so the loop can close without a human in it. What has been
missing is the judgment and the ride-back.

## Acceptance criteria

- **C1 — the judgment op.** A batch-only surface (`fleet judge
  [--day <date>] [--dry-run]`) reads a day's `route` rows and asks the
  sys1 lane — the same lane the router already selects with — to judge
  each selection against its outcome evidence (the workload, the
  attempts, the failovers, the turn's outcome where joinable). The
  verdict, its confidence, and the reason ride a new op (`route_judged`)
  keyed by the route row's identity. Every failure named; a lane that
  fails open answers with its own fallback prose, so the status is
  parsed and a non-ok judgment is a named refusal — no row, no
  influence, the recorded decision standing (the consuming-lanes rule).
- **C2 — the discipline (the port).** Batch-only: no turn's synchronous
  path references the module — pinned by a socket-guard test in slice
  17's shape. `--dry-run` makes no ask and shows what would be asked. A
  named daily budget (`JUDGE_BUDGET_USD_DAY`, with
  `FLEET_JUDGE_BUDGET_USD_DAY` able only to lower it) bounds the spend,
  and the spend record lands even when the row write fails (slice 17's
  sidecar pattern). The task's text never enters the judgment — the
  rows are redacted by design and the prompt says so.
- **C3 — the calibration surface.** `fleet calib --calibration` renders
  the per-provider judged quality as cards at the house bar: the
  judged count, the right/wrong split, the confidence-weighted
  wrong-rate, the cost per correct turn, and the **fit state** — the
  sibling's readiness rule ported (a provider's floor is trusted only
  when its judged sample carries enough rows and enough confidence
  span; the thresholds are named constants, stated for fleet's volume
  with the reasoning). An unfit provider renders as unfit with what it
  still needs, never as a score.
- **C4 — the ride-back, conservative and named.** The failover's
  preference consults the calibration: a provider whose
  confidence-weighted wrong-rate crosses a named floor AND whose sample
  is fit is skipped when a later candidate in the declared chain is
  healthy. The declared chain's order and the head are untouched — the
  influence moves only the failover's preference among the remaining
  candidates, and every skip names the calibration that caused it in
  the route row's reason. Config-gated (`calibration.informs_failover`,
  default on) with the floor named; a config that turns it off leaves
  the pre-slice behavior exactly.
- **C5 — the human's ground-truth seam.** A judged row can carry the
  owner's disposition (the sibling's vocabulary: agreed / disagreed /
  waived) through a named surface (`fleet judge --agree <key>` /
  `--disagree <key>`), and the disposition overrides the automated
  verdict in the calibration math. The judge is a signal, not an
  authority; the owner's grading is the ground truth.
- **C6 — the eval scenario.** A deterministic scenario grades the loop:
  the judged rows beside the routes, the calibration's fit rule, the
  conservative failover influence (a fit-and-wrong provider skipped, an
  unfit one not), the dry-run's no-spend, and the fail-open refusal —
  all through scripted seams, no paid live calls.
- **C7 — the UX at the house bar.** The calibration cards and the
  judgment's surface render visually (status dots, per-provider cards,
  the fit state named) — never a plain-text table — verified through
  the PTY capture plus a deterministic probe at 120x40 and 80x24.
- **C8 — the docs.** `docs/features/calibration-loop.md`: the loop end
  to end, the discipline, the fit rule and its thresholds with the
  reasoning, the ride-back's exact scope (what moves and what does
  not), the human seam, and PS-B with its combinations (PS-B + PS-A: a
  judged-wrong route gains its explanation; PS-B + PS6: the checked
  claim's routing is itself checked).
- **C9 — the gate.** The full suite green, nothing deleted,
  warning-free; every error or warning encountered lands in the ledger
  with its evidence.

## Design decisions

1. **The judgment is outcome-based, not text-based.** The rows are
   redacted by design, so the judge sees the workload, the attempts,
   and the outcomes — objective evidence — and the text-blindness is
   named in the doc as the limitation it is. The human seam (C5) is the
   ground truth for the cases the outcome cannot settle.
2. **The fit requirement outranks the influence.** Ported from the
   sibling's floor-fit readiness: no provider's routing behavior moves
   on a small or low-span sample. The thresholds are named constants
   sized for fleet's volume, with the reasoning stated.
3. **The influence is failover-only.** The owner's declared chain is a
   standing directive; the head is the head. The calibration moves
   only the preference among the remaining candidates, and the route
   row names the calibration that caused every skip.
4. **Native, not a dev-decisions runtime dependency** — the same rule
   slice 17 followed: the pattern ports, the machinery is fleet's own
   vendored sys1 lane over its MCP client.
5. **The judgment op is separate from the explanation op.** They share
   the discipline (batch-only, dry-run, budget, fail-open named,
   recoverable text) but not the surface: one judges decisions, the
   other explains verdicts. The shared discipline is factored where it
   is genuinely shared.

## Out of scope

- Judging the answer QUALITY (as opposed to the routing decision) —
  the explanation beat's subject beside the eval corpus's grades.
- The tabular lane's cost-vs-value scoring as a second judgment voice
  (named: sdm1 can score the spend curve once the rows accumulate).
- Cross-session calibration (the day's scope is the first closing).

## Assumptions (listed, not asked)

- The owner's declared chain order is untouched by this slice; the
  failover preference is the only routing surface the calibration
  moves, and it is config-gated with the gate defaulting on.
- The sys1 judgment's spend is small but real (d1 at the probed
  ~$0.04/1M input tokens); the budget bounds it and the dry-run
  rehearses it.
- The judged rows join the route rows by their identity in the shared
  stream, matching the devdec join shape (`sha256(x).hexdigest()[:16]`).
