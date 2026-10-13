"""
Core dataclasses shared across the sdm1 library.

The wire representation of a `TableResult` is the public contract (see
TECHNICAL-DOCUMENTATION.md §4): one normalized shape regardless of backend.
Adding keys is safe; renaming or removing is a breaking change.

Dataset contents never enter this module's output — predictions carry model
outputs (labels, probabilities), telemetry carries shapes and hashes only.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any, Literal

TaskType = Literal["classification", "regression", "forecast", "anomaly"]
Verdict = Literal["pass", "escalate", "block", "error"]


class Sdm1Error(Exception):
    """Base class for sdm1 errors raised out of the public API."""


class ProviderUnavailableError(Sdm1Error):
    """The backend's stack is missing or unconfigured. Message is actionable."""


class WorkerError(Sdm1Error):
    """The backend worker ran but failed (nonzero exit, bad JSON, GPU absent)."""


@dataclass
class TableTask:
    """
    A named structured-data prediction job.

    `target` is the column to predict. Rows carrying a non-null target are
    context (labeled examples for in-context learning); rows with a null or
    missing target are the query rows to score.

    `stype_overrides` maps column → semantic type (e.g. {"target":
    "categorical"}), passed through to the backend's stype inference.
    `confidence_floor` is expressed in the serving backend's own confidence
    semantics — in-context ensemble mean max-probability for nvidia-sdm.
    """

    id: str
    target: str
    task_type: TaskType = "classification"
    feature_columns: list[str] | None = None  # None = all columns except target
    stype_overrides: dict[str, str] = field(default_factory=dict)
    num_estimators: int = 8
    confidence_floor: float | None = None
    description: str = ""
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if self.task_type not in ("classification", "regression", "forecast", "anomaly"):
            raise ValueError(
                "task_type must be 'classification', 'regression', 'forecast' or 'anomaly', "
                f"got {self.task_type!r}"
            )
        if not self.target:
            raise ValueError("target is required")
        if self.num_estimators < 1:
            raise ValueError("num_estimators must be >= 1")
        if self.confidence_floor is not None and not 0.0 <= self.confidence_floor <= 1.0:
            raise ValueError("confidence_floor must be within [0, 1]")

    def to_spec(self) -> dict[str, Any]:
        """JSON-serializable request spec for the backend worker."""
        return {
            "target": self.target,
            "task_type": self.task_type,
            "feature_columns": list(self.feature_columns) if self.feature_columns else None,
            "stype_overrides": dict(self.stype_overrides),
            "num_estimators": self.num_estimators,
        }


@dataclass
class Prediction:
    """One scored query row. `row_index` is the index within the query rows.

    Classification: `label` is the predicted class, `probabilities` maps
    class → probability, `confidence` is the ensemble mean max-probability.
    Regression: `label` is the predicted value, both probability fields are
    None (never floor-gated — see interpret.py).
    Forecast: `label` is the point estimate (median), `extras["quantiles"]`
    maps level strings ("0.1".."0.9") to values.
    Anomaly: `label` is "normal"/"anomaly" by band membership and
    `confidence` is distribution coverage — the fraction of the predictive
    distribution consistent with the observed value (1.0 = central band
    already contains it; 0.0 = outside every predicted band).
    """

    row_index: int
    label: str | float | None
    probabilities: dict[str, float] | None = None
    confidence: float | None = None
    extras: dict[str, Any] | None = None

    def as_dict(self) -> dict[str, Any]:
        return {
            "row_index": self.row_index,
            "label": self.label,
            "probabilities": dict(self.probabilities) if self.probabilities else None,
            "confidence": self.confidence,
            "extras": dict(self.extras) if self.extras else None,
        }


# ── provider interface ────────────────────────────────────────────────────────

# (rows_context, rows_query, task, telemetry) -> list[Prediction].
# Implementations own subprocess/retry handling and telemetry population.
RunFn = Callable[[list[dict], list[dict], TableTask, dict], list[Prediction]]
AvailabilityFn = Callable[[], bool]


@dataclass
class Provider:
    """
    A pluggable structured-data backend.

    `run(rows_context, rows_query, task, telemetry) -> list[Prediction]` must
    return predictions already normalized to the Prediction wire shape.
    `is_configured` is the cheap check (paths/env vars exist); `is_available`
    actually probes the stack (may spawn a subprocess or make an HTTP call).
    `probe` optionally returns (available, reason) for doctor output — when
    absent, doctor falls back to is_available. Neither may import torch or
    `sdm` in-process — the core stays stdlib-only.
    """

    id: str
    name: str
    run: RunFn
    is_available: AvailabilityFn
    is_configured: AvailabilityFn | None = None
    probe: Callable[[], tuple[bool, str]] | None = None
    description: str = ""
    tier: str = "core"
    meta: dict[str, Any] = field(default_factory=dict)


# ── top-level result ──────────────────────────────────────────────────────────


@dataclass
class TableResult:
    """The public wire contract — one shape regardless of backend."""

    table: str
    task: str
    model: str
    models_used: list[str]
    task_type: str
    target: str
    n_context_rows: int
    n_query_rows: int
    num_estimators: int
    predictions: list[Prediction]
    telemetry: dict[str, Any] = field(default_factory=dict)
    verdict: Verdict = "pass"
    escalated: bool = False
    low_confidence_rows: list[int] = field(default_factory=list)
    latency_ms: int = 0
    input_sha256: str | None = None
    error: str | None = None
    reason: str | None = None
    summary: list[str] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        """JSON-serializable form for the JSONL log record and consumers.

        Key set is the public contract (TECHNICAL-DOCUMENTATION.md §4).
        Contains no dataset contents — only model outputs and telemetry.
        """
        return {
            "table": self.table,
            "task": self.task,
            "model": self.model,
            "models_used": list(self.models_used),
            "task_type": self.task_type,
            "target": self.target,
            "n_context_rows": self.n_context_rows,
            "n_query_rows": self.n_query_rows,
            "num_estimators": self.num_estimators,
            "predictions": [p.as_dict() for p in self.predictions],
            "telemetry": dict(self.telemetry),
            "verdict": self.verdict,
            "escalated": self.escalated,
            "low_confidence_rows": list(self.low_confidence_rows),
            "latency_ms": self.latency_ms,
            "input_sha256": self.input_sha256,
            "error": self.error,
            "reason": self.reason,
            "summary": list(self.summary),
        }
