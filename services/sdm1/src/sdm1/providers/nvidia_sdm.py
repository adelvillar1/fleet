"""
nvidia-sdm backend — NVIDIA Kumo-Tabular via the `sdm` package.

Talks to a dedicated `[models]` venv (Python ≥3.11, CUDA, `pip install
structured-data-models`) through a subprocess worker (`worker_sdm.py`), the
same pattern as sys1's `local` provider: the core never imports torch.

Configuration (env vars):
  - SDM1_MODELS_PYTHON  python executable inside the models venv
  - SDM1_MODELS_VENV    alternative: the venv root (bin/python is derived)
  - SDM1_TIMEOUT        worker timeout in seconds (default 900 — first run
                        may download model weights)
  - SDM1_WORKER_SCRIPT  override the worker script path (debug/testing hook)

Availability probing spawns `<models_python> -c "import sdm, torch ..."` and
caches the result briefly — probing is a subprocess, never an import.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path
from typing import Any

from ..parsers import parse_worker_output
from ..registry import register
from ..types import Prediction, Provider, ProviderUnavailableError, TableTask, WorkerError

WORKER_PATH = Path(__file__).with_name("worker_sdm.py")
DEFAULT_TIMEOUT_S = 900
PROBE_TIMEOUT_S = 120
PROBE_TTL_S = 60

_probe_cache: dict[str, tuple[float, tuple[bool, str]]] = {}


def _env(name: str, default: str | None = None) -> str | None:
    return os.environ.get(name, default)


def models_python() -> str | None:
    """Locate the python executable inside the [models] venv, or None."""
    explicit = _env("SDM1_MODELS_PYTHON")
    if explicit:
        return explicit
    venv_root = _env("SDM1_MODELS_VENV")
    if venv_root:
        candidate = Path(venv_root).expanduser() / "bin" / "python"
        return str(candidate)
    return None


def is_configured() -> bool:
    return models_python() is not None


def probe_stack(models_python_path: str) -> tuple[bool, str]:
    """Probe torch/sdm/CUDA by subprocess. Returns (available, reason)."""
    script = (
        "import json,sys;"
        "try:"
        " import torch;"
        " info={'torch': torch.__version__, 'cuda': bool(torch.cuda.is_available())};"
        " try:"
        "  import sdm; info['sdm']=getattr(sdm,'__version__','unknown')"
        " except Exception as e: info['sdm_error']=str(e)[:200]"
        " print(json.dumps(info))"
        "except Exception as e:"
        " print(json.dumps({'import_error': str(e)[:200]}))"
    )
    try:
        proc = subprocess.run(
            [models_python_path, "-c", script],
            capture_output=True,
            text=True,
            timeout=PROBE_TIMEOUT_S,
        )
    except (OSError, subprocess.TimeoutExpired) as e:
        return False, f"probe failed to run: {e}"
    if proc.returncode != 0:
        return False, f"probe exited {proc.returncode}: {(proc.stderr or '').strip()[:200]}"
    try:
        info = json.loads(proc.stdout.strip().splitlines()[-1])
    except (json.JSONDecodeError, ValueError):
        return False, "probe produced unparseable output"
    if "import_error" in info:
        return False, f"stack import failed: {info['import_error']}"
    if not info.get("cuda", False):
        return False, "CUDA not available (sdm backends require a CUDA GPU)"
    if "sdm_error" in info:
        return False, f"sdm import failed: {info['sdm_error']}"
    parts = [f"torch {info.get('torch', '?')}", f"sdm {info.get('sdm', '?')}", "cuda ok"]
    return True, "; ".join(parts)


def probe_detail() -> tuple[bool, str]:
    """(available, reason) with the probe cache applied — used by doctor."""
    mp = models_python()
    if mp is None:
        return False, "not configured — set SDM1_MODELS_PYTHON (or SDM1_MODELS_VENV)"
    now = time.monotonic()
    cached = _probe_cache.get(mp)
    if cached and now - cached[0] < PROBE_TTL_S:
        return cached[1]
    result = probe_stack(mp)
    _probe_cache[mp] = (now, result)
    return result


def is_available() -> bool:
    return probe_detail()[0]


def reset_probe_cache() -> None:
    _probe_cache.clear()


def _worker_script() -> str:
    return _env("SDM1_WORKER_SCRIPT") or str(WORKER_PATH)


def run(
    rows_context: list[dict],
    rows_query: list[dict],
    task: TableTask,
    telemetry: dict[str, Any],
) -> list[Prediction]:
    mp = models_python()
    if mp is None:
        raise ProviderUnavailableError(
            "nvidia-sdm backend is not configured: set SDM1_MODELS_PYTHON (or SDM1_MODELS_VENV) "
            "to the python inside your sdm1[models] venv — create it with "
            "`python3.11 -m venv <dir> && <dir>/bin/pip install 'sdm1[models]'` on a CUDA machine."
        )
    request = {
        "model": "nvidia/Kumo-Tabular",
        **task.to_spec(),
        "rows_context": rows_context,
        "rows_query": rows_query,
    }
    timeout = float(_env("SDM1_TIMEOUT") or DEFAULT_TIMEOUT_S)
    t0 = time.monotonic()
    try:
        proc = subprocess.run(
            [mp, _worker_script()],
            input=json.dumps(request, default=str),
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as e:
        telemetry.update({"error": f"worker timeout after {timeout:.0f}s", "error_kind": "timeout"})
        raise WorkerError(f"nvidia-sdm worker timed out after {timeout:.0f}s") from e
    except OSError as e:
        telemetry.update({"error": str(e)[:300], "error_kind": "spawn"})
        raise WorkerError(f"failed to spawn models venv python: {e}") from e

    elapsed_ms = int((time.monotonic() - t0) * 1000)
    telemetry.update(
        {
            "worker_returncode": proc.returncode,
            "worker_latency_ms": elapsed_ms,
            "worker_stderr_tail": (proc.stderr or "").strip()[-400:],
        }
    )
    if proc.returncode != 0:
        telemetry["error_kind"] = "worker_exit"
        tail = (proc.stderr or proc.stdout or "").strip()[-300:]
        raise WorkerError(f"nvidia-sdm worker exited {proc.returncode}: {tail}")

    predictions, structured_ok, error = parse_worker_output(proc.stdout)
    telemetry["structured_ok"] = structured_ok
    if not structured_ok:
        telemetry["error_kind"] = "parse"
        raise WorkerError(f"nvidia-sdm worker output unparseable: {error}")
    return predictions


register(
    Provider(
        id="nvidia-sdm",
        name="NVIDIA SDM (Kumo-Tabular)",
        run=run,
        is_available=is_available,
        is_configured=is_configured,
        probe=probe_detail,
        description="In-context tabular foundation model (nvidia/Kumo-Tabular) via the sdm package in a [models] venv.",
        tier="core",
        meta={
            "model_class": "in-context",
            "weights": "nvidia/Kumo-Tabular",
            "weights_license": "OpenMDW-1.1",
            "code_license": "Apache-2.0",
            "config_hint": "set SDM1_MODELS_PYTHON (or SDM1_MODELS_VENV)",
        },
    )
)
