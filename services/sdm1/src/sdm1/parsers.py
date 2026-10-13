"""
Normalize backend worker output into Prediction wire shapes.

The worker protocol is ours, but the worker itself wraps third-party model
outputs whose shapes vary (classification ensembles, regression values,
different class-name sources). Parsers stay tolerant: several key aliases,
list-form entries, and derived confidence when the worker omits one.

Parsing never raises — malformed output degrades to structured_ok=False with
an error snippet, which the API turns into an error verdict.
"""

from __future__ import annotations

import json
from typing import Any

from .types import Prediction

_PROB_KEYS = ("probabilities", "probs", "probs_by_class", "class_probabilities")
_LABEL_KEYS = ("label", "prediction", "pred", "value", "y_pred")
_CONF_KEYS = ("confidence", "conf", "score")


def parse_worker_output(raw: str) -> tuple[list[Prediction], bool, str | None]:
    """Parse worker stdout. Returns (predictions, structured_ok, error).

    `structured_ok` reflects whether we got a well-formed JSON envelope with
    predictions — mirroring sys1's structured_ok telemetry key so dashboards
    can track parse failures the same way.
    """
    try:
        payload = json.loads(raw)
    except (json.JSONDecodeError, ValueError):
        snippet = raw[:200].replace("\n", " ") if raw else "(empty)"
        return [], False, f"worker output is not JSON: {snippet}"
    if not isinstance(payload, dict):
        return [], False, "worker output is not a JSON object"

    if payload.get("ok") is False:
        return [], False, str(payload.get("error") or "worker reported failure")

    entries = payload.get("predictions")
    if entries is None:
        return [], False, "worker output missing 'predictions' key"
    if not isinstance(entries, list):
        return [], False, "'predictions' is not a list"

    predictions: list[Prediction] = []
    for i, entry in enumerate(entries):
        pred = _normalize_entry(entry, default_row=i)
        if pred is not None:
            predictions.append(pred)
    return predictions, True, None


def _normalize_entry(entry: Any, *, default_row: int) -> Prediction | None:
    """One prediction entry; tolerate dict and [row_index, label, probs] forms."""
    row = default_row
    label: Any = None
    probs: dict[str, float] | None = None
    conf: float | None = None

    if isinstance(entry, list):
        if len(entry) >= 2:
            row, label = entry[0], entry[1]
        if len(entry) >= 3 and isinstance(entry[2], dict):
            probs = entry[2]
    elif isinstance(entry, dict):
        row = entry.get("row_index", entry.get("index", default_row))
        for key in _LABEL_KEYS:
            if key in entry and entry[key] is not None:
                label = entry[key]
                break
        for key in _PROB_KEYS:
            if isinstance(entry.get(key), dict):
                probs = entry[key]
                break
        for key in _CONF_KEYS:
            if entry.get(key) is not None:
                conf = _as_float(entry[key])
                break
    else:
        return None

    if probs is not None:
        probs = {str(k): _as_float(v) for k, v in probs.items() if _as_float(v) is not None}
        if not probs:
            probs = None
    if label is not None and not isinstance(label, (str, int, float, bool)):
        label = str(label)
    if conf is None and probs:
        conf = max(probs.values())  # ensemble mean max-probability for classification
    try:
        row = int(row)
    except (TypeError, ValueError):
        row = default_row
    return Prediction(row_index=row, label=label, probabilities=probs, confidence=conf)


def _as_float(value: Any) -> float | None:
    try:
        f = float(value)
    except (TypeError, ValueError):
        return None
    return None if f != f else f  # NaN guard


# ── hosted predict-response parsing (tabpfn-hosted) ──────────────────────────

# Quantile levels the hosted service returns, level index 4 == median.
QUANTILE_LEVELS = ("0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9")


def parse_predict_response(
    payload: Any,
    task_type: str,
    rows_query: list[dict] | None = None,
    actual_column: str | None = None,
) -> tuple[list[Prediction], bool, str | None]:
    """Normalize a hosted /tabpfn/predict response into Predictions.

    Tolerant of the shapes the service returns (see TECHNICAL-DOCUMENTATION §6):
      - classification: `prediction` is a list of per-row probability lists
        aligned with the top-level `classes` list.
      - regression: `prediction` is a list of floats (median/mean/mode).
      - forecast / anomaly: `prediction` is levels x rows (output_type
        "quantiles") or a dict with mean/median/quantiles ("full"/"main").
    Anomaly rows compare the observed value (from `actual_column` in
    `rows_query`) against the band: label normal/anomaly, confidence =
    distribution coverage.
    """
    if not isinstance(payload, dict):
        return [], False, "predict response is not a JSON object"
    pred = payload.get("prediction")
    if pred is None:
        return [], False, "predict response missing 'prediction' key"

    if task_type == "classification":
        meta = payload.get("metadata") or {}
        classes = payload.get("classes") or meta.get("classes")
        if not isinstance(pred, list) or not isinstance(classes, list) or not classes:
            return [], False, "classification prediction requires a prediction list and classes"
        predictions: list[Prediction] = []
        for i, probs_list in enumerate(pred):
            if not isinstance(probs_list, list):
                continue
            probs = {str(c): _as_float(p) for c, p in zip(classes, probs_list, strict=False)}
            probs = {k: v for k, v in probs.items() if v is not None}
            if not probs:
                continue
            label = max(probs, key=probs.__getitem__)
            predictions.append(
                Prediction(row_index=i, label=label, probabilities=probs, confidence=probs[label])
            )
        return predictions, True, None

    if task_type == "regression":
        values = _per_row_values(pred)
        if values is None:
            return [], False, "regression prediction is not a value list"
        return (
            [Prediction(row_index=i, label=v) for i, v in enumerate(values) if v is not None],
            True,
            None,
        )

    # forecast / anomaly: quantile bands, levels x rows
    levels = _quantile_matrix(pred)
    if levels is None:
        return [], False, "forecast/anomaly prediction carries no quantile matrix"
    n_rows = len(levels[0])
    predictions = []
    for i in range(n_rows):
        band = {QUANTILE_LEVELS[j]: levels[j][i] for j in range(len(levels)) if levels[j][i] is not None}
        median = band.get("0.5", band.get("0.9"))
        if task_type == "forecast":
            predictions.append(
                Prediction(row_index=i, label=median, extras={"quantiles": band})
            )
            continue
        actual = None
        if rows_query and actual_column:
            actual = _as_float((rows_query[i] or {}).get(actual_column)) if i < len(rows_query) else None
        if actual is None:
            predictions.append(Prediction(row_index=i, label=median, extras={"quantiles": band}))
            continue
        coverage, inside = _coverage(band, actual)
        predictions.append(
            Prediction(
                row_index=i,
                label="normal" if inside else "anomaly",
                confidence=coverage,
                extras={"quantiles": band, "actual": actual},
            )
        )
    return predictions, True, None


def _per_row_values(pred: Any) -> list[float | None] | None:
    """Regression shapes: list of floats, or dict with median/mean lists."""
    if isinstance(pred, list):
        vals = [_as_float(v) for v in pred]
        return vals if any(v is not None for v in vals) else None
    if isinstance(pred, dict):
        for key in ("median", "mean", "mode"):
            if isinstance(pred.get(key), list):
                return [_as_float(v) for v in pred[key]]
    return None


def _quantile_matrix(pred: Any) -> list[list[float | None]] | None:
    """Quantiles as levels x rows; accepts quantiles output (levels x rows list)
    and full/main output ({"quantiles": levels x rows})."""
    if isinstance(pred, dict):
        pred = pred.get("quantiles")
    if not isinstance(pred, list) or not pred:
        return None
    matrix = [[_as_float(v) for v in row_vals] if isinstance(row_vals, list) else [] for row_vals in pred]
    widths = {len(r) for r in matrix}
    if len(widths) != 1 or not widths or 0 in widths:
        return None
    return matrix


def _coverage(band: dict[str, float], actual: float) -> tuple[float, bool]:
    """Distribution coverage of `actual`: the fraction of the predictive
    distribution inside the tightest symmetric quantile pair containing it.

    The service returns level strings "0.1".."0.9"; the symmetric pair
    (0.5-d, 0.5+d) covers a fraction 2d of the distribution, so the widest
    returned band (0.1-0.9) covers 0.8 and anything outside it scores 0.0.
    """
    for d in (0.4, 0.3, 0.2, 0.1):
        lo = band.get(f"{0.5 - d:.1f}")
        hi = band.get(f"{0.5 + d:.1f}")
        if lo is not None and hi is not None and lo <= actual <= hi:
            return round(2 * d, 2), True
    return 0.0, False
