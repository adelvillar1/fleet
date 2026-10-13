"""
JSONL logging and calibration helpers.

Same on-disk convention as sys1: `<log_dir>/YYYY/MM/DD/events.jsonl`, one
record per run, joined back via `input_sha256`. sdm1's hard rule on top:
records never contain dataset contents — only shapes, hashes, model outputs,
and telemetry. `DEFAULT_REDACT` drops anything that smells like raw rows if
it ever sneaks into a record.
"""

from __future__ import annotations

import copy
import json
import math
from collections import defaultdict
from collections.abc import Iterable
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .types import Prediction

DEFAULT_REDACT = ("rows", "rows_context", "rows_query", "cells", "values", "text", "content")


def log_dir() -> Path:
    base = Path(_env("SDM1_LOG_DIR") or str(Path.home() / ".local" / "share" / "sdm1" / "logs"))
    return base.expanduser() / datetime.now(timezone.utc).strftime("%Y/%m/%d")


def _env(name: str) -> str | None:
    import os

    return os.environ.get(name)


def log_record(record: dict, *, redact: Iterable[str] = DEFAULT_REDACT) -> Path | None:
    """Append one JSONL record. Returns the path, or None if the write failed.

    Logging must never raise — a broken disk must not break the caller's run.
    """
    try:
        base = log_dir()
        base.mkdir(parents=True, exist_ok=True)
        path = base / "events.jsonl"
        rec = copy.deepcopy(record)
        rec.setdefault("ts", datetime.now(timezone.utc).isoformat())
        for field_name in redact:
            _redact_field(rec, field_name)
        with open(path, "a") as f:
            f.write(json.dumps(rec, default=str) + "\n")
        return path
    except Exception:
        return None


def _redact_field(obj: Any, field_name: str) -> None:
    if isinstance(obj, dict):
        for k, v in list(obj.items()):
            if k == field_name:
                obj[k] = "[redacted]"
            else:
                _redact_field(v, field_name)
    elif isinstance(obj, list):
        for item in obj:
            _redact_field(item, field_name)


def confidence_histogram(predictions: list[Prediction], *, bins: int = 10) -> dict[str, list[int]]:
    """Bucket prediction confidences into `[0, 1/bins, ..., 1]` for the dashboard."""
    buckets: dict[str, list[int]] = defaultdict(lambda: [0] * bins)
    for pred in predictions:
        conf = pred.confidence
        if conf is None:
            continue
        try:
            c = float(conf)
        except (TypeError, ValueError):
            continue
        if c != c:  # NaN
            continue
        idx = min(bins - 1, max(0, math.floor(c * bins)))
        buckets["all"][idx] += 1
    return dict(buckets)
