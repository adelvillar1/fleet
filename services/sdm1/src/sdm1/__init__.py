"""sdm1 — structured-data model abstraction.

One Python API over structured-data (tabular/relational) foundation
models — NVIDIA Kumo-Tabular first — with a normalized result shape
(`TableResult`, the public contract) and JSONL telemetry. Stdlib-only
core; the torch-backed backends live behind the ``[models]`` extra in a
dedicated venv that providers reach via ``subprocess``, never an import.
"""

from . import providers as _providers  # noqa: F401  (import registers the built-in backends)
from .api import classify, classify_csv, health_report
from .registry import all_ids, get, register
from .types import (
    Prediction,
    Provider,
    ProviderUnavailableError,
    Sdm1Error,
    TableResult,
    TableTask,
    WorkerError,
)

__version__ = "0.1.0"

__all__ = [
    "Prediction",
    "Provider",
    "ProviderUnavailableError",
    "Sdm1Error",
    "TableResult",
    "TableTask",
    "WorkerError",
    "__version__",
    "all_ids",
    "classify",
    "classify_csv",
    "get",
    "health_report",
    "register",
]
