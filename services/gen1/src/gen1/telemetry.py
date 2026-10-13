"""Structurally redacted telemetry. A row carries counts and identifiers — input
sha256, char/byte counts, provider, model, duration, request_id — and never
content: no input text, no audio bytes, no key material. The redaction test
(C4) reads the default sink's actual rows back and proves it.

The sink is one JSONL append per attempt, failed legs included — a leg that
failed after submission was billed, so its row is where the charge is
reconcilable. Library callers may override the sink; a broken sink must never
break media, so recording is best-effort.
"""

from __future__ import annotations

import contextlib
import hashlib
import json
import os
from collections.abc import Callable
from pathlib import Path

from .types import TelemetryRow

DEFAULT_SINK = Path.home() / ".config" / "gen1" / "telemetry.jsonl"

_sink: Path | Callable[[TelemetryRow], None] | None = None


def sink_path() -> Path:
    """The default sink: GEN1_TELEMETRY_FILE when set, else
    ~/.config/gen1/telemetry.jsonl (chmod 600, beside the key file, never
    committed)."""
    env = os.environ.get("GEN1_TELEMETRY_FILE")
    return Path(env).expanduser() if env else DEFAULT_SINK


def set_sink(sink: Path | Callable[[TelemetryRow], None] | None) -> None:
    """Override the sink: a path, a callable taking the row, or None to restore
    the default file. Tests point it at a tmp path."""
    global _sink
    _sink = sink


def build_row(
    *,
    payload: bytes | str,
    provider: str,
    model: str,
    duration_seconds: float,
    request_id: str | None,
) -> TelemetryRow:
    """Build the redacted row for one attempt. The payload is hashed and counted,
    never stored."""
    if isinstance(payload, str):
        raw = payload.encode("utf-8")
        chars, size = len(payload), len(raw)
    else:
        raw = payload
        chars, size = 0, len(payload)
    return TelemetryRow(
        input_sha256=hashlib.sha256(raw).hexdigest(),
        input_chars=chars,
        input_bytes=size,
        provider=provider,
        model=model,
        duration_seconds=round(float(duration_seconds), 3),
        request_id=request_id,
    )


def record(row: TelemetryRow) -> None:
    """Append one row to the sink. Best-effort: a sink that cannot be written is
    never allowed to fail the media call that produced the row."""
    try:
        sink = _sink
        if sink is None:
            path = sink_path()
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open("a", encoding="utf-8") as fh:
                fh.write(json.dumps(row.__dict__, sort_keys=True) + "\n")
            with contextlib.suppress(OSError):
                os.chmod(path, 0o600)
        elif callable(sink):
            sink(row)
        else:
            path = Path(sink).expanduser()
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open("a", encoding="utf-8") as fh:
                fh.write(json.dumps(row.__dict__, sort_keys=True) + "\n")
    except Exception:  # telemetry never breaks the call
        pass
