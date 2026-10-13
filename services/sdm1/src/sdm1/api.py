"""
Top-level public API.

`sdm1.classify(...)` is the one function most callers need. It composes:
row split → provider resolution → backend run → interpretation → JSONL.

Rows carrying a non-null `target` are context (labeled in-context examples);
rows with a null/missing target are the query rows that get scored.
"""

from __future__ import annotations

import csv
import hashlib
import json
import time
from typing import Any

from . import providers as _providers  # noqa: F401  (import registers built-ins)
from .interpret import effective_floor, interpret
from .registry import REGISTRY, get
from .telemetry import confidence_histogram, log_record
from .types import Prediction, Sdm1Error, TableResult, TableTask


def classify(
    table: list[dict],
    *,
    target: str | None = None,
    task_type: str = "classification",
    task: TableTask | None = None,
    model: str = "nvidia-sdm",
    stype_overrides: dict[str, str] | None = None,
    num_estimators: int = 8,
    confidence_floor: float | None = None,
    table_name: str = "table",
    log: bool = True,
) -> TableResult:
    """
    Score the unlabeled rows of `table` using its labeled rows as context.

    Args:
        table: rows as dicts (pandas users: `df.to_dict("records")`).
        target: the column to predict. Required unless `task` is given.
        task_type: "classification" (default) or "regression".
        task: a full `TableTask` — overrides target/task_type/num_estimators.
        model: backend id from the registry (default "nvidia-sdm").
        stype_overrides: column → semantic type, passed to the backend's
            stype inference.
        num_estimators: ensemble size for in-context backends.
        confidence_floor: request-level floor override (expressed in the
            backend's own confidence semantics — see interpret.py).
        table_name: identifier written into the result/log.
        log: append a JSONL record (the calibration set).

    Returns:
        TableResult — the public wire contract (TECHNICAL-DOCUMENTATION §4).

    Raises:
        ValueError: malformed input (no target, no context/query rows).
        The backend never raises out: failures come back as a TableResult
        with verdict="error" (fail-open, mirroring sys1).
    """
    if task is None:
        if not target:
            raise ValueError("target is required (or pass a full TableTask as `task=`)")
        task = TableTask(
            id=f"{target}:{task_type}",
            target=target,
            task_type=task_type,  # type: ignore[arg-type]
            stype_overrides=dict(stype_overrides or {}),
            num_estimators=num_estimators,
            confidence_floor=confidence_floor,
        )
    else:
        if confidence_floor is not None:
            task = TableTask(
                id=task.id,
                target=task.target,
                task_type=task.task_type,
                feature_columns=task.feature_columns,
                stype_overrides=task.stype_overrides,
                num_estimators=task.num_estimators,
                confidence_floor=confidence_floor,
                description=task.description,
                metadata=task.metadata,
            )

    rows_context = [r for r in table if r.get(task.target) is not None]
    rows_query = [r for r in table if r.get(task.target) is None]
    if not rows_context:
        raise ValueError(
            f"no context rows: every row has a null/missing {task.target!r}. "
            "In-context learning needs labeled example rows."
        )
    if not rows_query:
        raise ValueError(
            f"no query rows: every row already carries {task.target!r}. "
            "Leave the target null (or omit the key) on the rows you want scored."
        )

    input_sha256 = hashlib.sha256(
        json.dumps(rows_query, sort_keys=True, default=str).encode("utf-8")
    ).hexdigest()

    t0 = time.monotonic()
    telemetry: dict[str, Any] = {}
    models_used: list[str] = []
    predictions: list[Prediction] = []
    error: str | None = None
    reason: str | None = None

    try:
        provider = get(model)
        predictions = provider.run(rows_context, rows_query, task, telemetry)
        models_used = [provider.id]
        if not predictions:
            reason = "backend returned zero predictions"
    except Sdm1Error as e:
        error = str(e)
        reason = f"{type(e).__name__}: {str(e)[:300]}"
    except Exception as e:  # fail-open: a broken backend never breaks the caller
        error = f"{type(e).__name__}: {e}"
        reason = f"provider failure: {str(e)[:300]}"

    elapsed_ms = int((time.monotonic() - t0) * 1000)

    if error or not predictions:
        result = TableResult(
            table=table_name,
            task=task.id,
            model=model,
            models_used=models_used,
            task_type=task.task_type,
            target=task.target,
            n_context_rows=len(rows_context),
            n_query_rows=len(rows_query),
            num_estimators=task.num_estimators,
            predictions=[],
            telemetry=telemetry,
            verdict="error",
            escalated=True,
            latency_ms=elapsed_ms,
            input_sha256=input_sha256,
            error=error or reason,
            reason=reason,
            summary=[f"run failed — {reason}"],
        )
    else:
        verdict, escalated, low_rows, summary = interpret(predictions, model, task)
        result = TableResult(
            table=table_name,
            task=task.id,
            model=model,
            models_used=models_used,
            task_type=task.task_type,
            target=task.target,
            n_context_rows=len(rows_context),
            n_query_rows=len(rows_query),
            num_estimators=task.num_estimators,
            predictions=predictions,
            telemetry={
                **telemetry,
                "confidence_floor": effective_floor(model, task),
                "confidence_histogram": confidence_histogram(predictions),
            },
            verdict=verdict,
            escalated=escalated,
            low_confidence_rows=low_rows,
            latency_ms=elapsed_ms,
            input_sha256=input_sha256,
            summary=summary,
        )

    if log:
        log_record({"op": "classify", **result.as_dict()})
    return result


def classify_csv(path: str, **kwargs: Any) -> TableResult:
    """Read a CSV (stdlib csv module) and run classify over its rows.

    Cells are coerced: "" → None, true/false → bool, ints and floats parsed —
    everything else stays a string.
    """
    rows = _read_csv(path)
    return classify(rows, **kwargs)


def _read_csv(path: str) -> list[dict]:
    with open(path, newline="", encoding="utf-8") as f:
        return [{k: _coerce(v) for k, v in row.items()} for row in csv.DictReader(f)]


def _coerce(value: str | None) -> Any:
    if value is None or value == "":
        return None
    if value.lower() == "true":
        return True
    if value.lower() == "false":
        return False
    for cast in (int, float):
        try:
            return cast(value)
        except ValueError:
            continue
    return value


def health_report() -> dict[str, dict[str, Any]]:
    """Per-backend availability snapshot for `sdm1 doctor`."""
    report: dict[str, dict[str, Any]] = {}
    for pid in sorted(REGISTRY):
        provider = REGISTRY[pid]
        configured = provider.is_configured() if provider.is_configured else None
        if configured is False:
            hint = provider.meta.get("config_hint") or "see the provider's configuration docs"
            available, reason = False, f"not configured — {hint}"
        elif provider.probe is not None:
            available, reason = provider.probe()
        else:
            from .providers.nvidia_sdm import probe_detail

            available, reason = probe_detail()
        report[pid] = {
            "name": provider.name,
            "configured": configured,
            "available": available,
            "reason": reason,
            "meta": dict(provider.meta),
        }
    return report
