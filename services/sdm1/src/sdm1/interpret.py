"""
Confidence gating and verdict interpretation.

Policy half of the library: takes normalized predictions from a run and
turns them into a verdict plus the set of rows below the confidence floor.

Confidence semantics are per model class and must not be thresholded with
the same number (sys1's Jev-vs-Decide lesson, carried over):

  - In-context backends (nvidia-sdm / KumoTabular): confidence is the
    ensemble mean max-class-probability over `num_estimators` passes. It is
    a distribution-shape statistic — smoother and typically higher than a
    single-pass max-probability. A 0.75 here does not mean what a 0.75 from
    a single-pass fit/predict head means.
  - Hosted fit/predict backends (tabpfn-hosted): the serving model's own
    class probability for the winning class (single head from the caller's
    view, ensemble inside the service). Closer to a literal "x% sure" than
    the in-context mean — never share a floor with nvidia-sdm.
  - Anomaly rows (any backend): confidence is distribution coverage — the
    fraction of the predictive distribution consistent with the observed
    value (1.0 = central band contains it, 0.0 = outside every band). The
    floor therefore reads "at least this much of the distribution must
    agree with the observation".

Floors are therefore keyed per backend id, never shared. Precedence:
request-level override > task.confidence_floor > _DEFAULT_FLOORS[model].

Regression and forecast rows carry no confidence — they are never
floor-gated (a missing gateable number is "cannot verify", not "escalate"),
mirroring sys1's answer_confidence contract.
"""

from __future__ import annotations

from .types import Prediction, TableTask, Verdict

# Default confidence floors per backend id. Tune per backend — never reuse a
# number across backends expecting it to mean the same thing.
_DEFAULT_FLOORS = {
    "nvidia-sdm": 0.6,  # in-context ensemble; start low, calibrate from telemetry
    "tabpfn-hosted": 0.6,  # hosted single-head; start low, calibrate from telemetry
}


def effective_floor(model_id: str, task: TableTask | None = None, *, floor_override: float | None = None) -> float:
    """Resolve the confidence floor for one run.

    Precedence, top wins:
      1. request-level `floor_override`
      2. the task's own `confidence_floor`
      3. `_DEFAULT_FLOORS[model_id]` (falling back to 0.6 for unknown ids)
    """
    if floor_override is not None:
        return float(floor_override)
    if task is not None and task.confidence_floor is not None:
        return float(task.confidence_floor)
    return _DEFAULT_FLOORS.get(model_id, 0.6)


def interpret(
    predictions: list[Prediction],
    model_id: str,
    task: TableTask | None = None,
    *,
    floor_override: float | None = None,
) -> tuple[Verdict, bool, list[int], list[str]]:
    """
    Apply the per-backend floor across query-row predictions.

    Returns (verdict, escalated, low_confidence_rows, summary). Only rows
    with a confidence below the floor escalate; rows without a confidence
    (regression) never gate. Rows with a missing label escalate regardless —
    a prediction that isn't one is the worst outcome.
    """
    floor = effective_floor(model_id, task, floor_override=floor_override)
    low_rows: list[int] = []
    summary: list[str] = [f"floor: {floor:.2f} (model {model_id})"]
    for pred in predictions:
        if pred.label is None:
            low_rows.append(pred.row_index)
            summary.append(f"  row {pred.row_index}: (no label) — escalate")
            continue
        if pred.confidence is not None and pred.confidence < floor:
            low_rows.append(pred.row_index)
            summary.append(
                f"  row {pred.row_index}: {str(pred.label)[:40]} (conf {pred.confidence:.2f} < {floor:.2f})"
            )
    escalated = bool(low_rows)
    verdict: Verdict = "escalate" if escalated else "pass"
    if not escalated:
        summary.append(f"  all {len(predictions)} query rows clear the floor")
    return verdict, escalated, low_rows, summary
