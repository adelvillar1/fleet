"""JSONL telemetry for sem1 — same discipline as sys1/sdm1/devdec.

One row per operation under ~/.local/share/sem1/logs/YYYY/MM/DD.jsonl.

Redaction is structural, not conventional: `record()` drops any top-level key
in FORBIDDEN and callers pass only metadata. Input text and key material must
never reach a row; the selftest proves it by grepping the raw file.
"""

from __future__ import annotations

import hashlib
import json
import os
import time
from datetime import datetime, timezone
from pathlib import Path

LOG_DIR = Path(os.environ["SEM1_LOG_DIR"]) if os.environ.get("SEM1_LOG_DIR") else (
    Path.home() / ".local" / "share" / "sem1" / "logs"
)

# Top-level keys a telemetry row may never carry. `record()` strips them even
# if a caller leaks them in — the guarantee lives here, not at call sites.
FORBIDDEN = ("text", "input", "inputs", "prompt", "key", "api_key", "vectors")


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8", "replace")).hexdigest()


def record(event: dict, log_dir: Path | None = None) -> dict:
    """Append one telemetry row; returns the row as written."""
    log_dir = Path(log_dir) if log_dir else LOG_DIR
    clean = {k: v for k, v in event.items() if k not in FORBIDDEN}
    clean.setdefault("ts", datetime.now(timezone.utc).isoformat())
    day = datetime.now(timezone.utc).strftime("%Y/%m/%d")
    path = log_dir / f"{day}.jsonl"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(clean, default=str) + "\n")
    return clean


class Timed:
    """Millisecond timer: t = Timed(); ...; ms = t.stop()."""

    def __init__(self) -> None:
        self._t0 = time.perf_counter()

    def stop(self) -> float:
        return (time.perf_counter() - self._t0) * 1000.0
