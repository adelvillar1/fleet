"""
tabpfn-hosted backend — Prior Labs TabPFN foundation models over their REST API.

The second backend pattern alongside the `[models]` venv: plain HTTPS with
stdlib `urllib` (Bearer token auth), no torch, no venv, no subprocess. The
core stays stdlib-only; the wire flow is (live-verified 2026-10-07):

  POST /tabpfn/prepare_train_set_upload  {x_train_info, y_train_info}
  PUT  x_train.csv / y_train.csv         → signed URLs (GCS) + required headers
  POST /tabpfn/fit                       {train_set_upload_id, task} → fitted_train_set_id
  POST /tabpfn/prepare_test_set_upload   {fitted_train_set_id, x_test_info}
  PUT  x_test.csv                        → signed URL
  POST /tabpfn/predict                   {test_set_upload_id, fitted_train_set_id, task_config}

Task-type mapping ( TECHNICAL-DOCUMENTATION §6 ):
  classification → fit "classification", predict output_type "probas"
  regression     → fit "regression",        predict output_type "median"
  forecast       → fit "regression",        predict output_type "quantiles"
  anomaly        → fit "regression",        predict output_type "quantiles";
                   query rows carry the observed value in
                   `task.metadata["actual_column"]` (never sent as a feature)
                   and get normal/anomaly labels + coverage confidence.

Configuration (env vars):
  - TABPFN_API_KEY        Prior Labs API key (required; never logged)
  - SDM1_TABPFN_BASE      API base (default https://api.priorlabs.ai)
  - SDM1_TABPFN_TIMEOUT   per-request timeout seconds (default 600 — small
                          tasks measured ~160s end-to-end including fit queue)

Hard rules carried: dataset cell values go to the signed upload URLs only
and never into telemetry or logs; the key value is never logged; failures
surface as Sdm1Error subclasses so the API fails open.
"""

from __future__ import annotations

import contextlib
import csv
import io
import json
import os
import time
import urllib.error
import urllib.request
from typing import Any

from ..parsers import parse_predict_response
from ..registry import register
from ..types import Prediction, Provider, ProviderUnavailableError, TableTask, WorkerError

DEFAULT_BASE = "https://api.priorlabs.ai"
DEFAULT_TIMEOUT_S = 600
PROBE_TIMEOUT_S = 30
PROBE_TTL_S = 60

_probe_cache: tuple[float, tuple[bool, str]] | None = None

# fit task + predict output_type per sdm1 task_type
_WIRE_MAP = {
    "classification": ("classification", "probas"),
    "regression": ("regression", "median"),
    "forecast": ("regression", "quantiles"),
    "anomaly": ("regression", "quantiles"),
}


def _env(name: str, default: str | None = None) -> str | None:
    return os.environ.get(name, default)


def base_url() -> str:
    return (_env("SDM1_TABPFN_BASE") or DEFAULT_BASE).rstrip("/")


def api_key() -> str | None:
    key = _env("TABPFN_API_KEY")
    return key.strip() if key and key.strip() else None


def is_configured() -> bool:
    return api_key() is not None


def _authed_json(path: str, payload: dict[str, Any] | None, timeout: float) -> dict[str, Any]:
    """One API call. Raises ProviderUnavailableError for auth/quota, WorkerError otherwise."""
    key = api_key()
    if key is None:
        raise ProviderUnavailableError(
            "tabpfn-hosted backend is not configured: set TABPFN_API_KEY to your Prior Labs "
            "API key (https://ux.priorlabs.ai) — the core stays stdlib-only; no SDK is needed."
        )
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    headers = {"Authorization": f"Bearer {key}"}
    if data is not None:
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(
        f"{base_url()}{path}",
        data=data,
        headers=headers,
        method="POST" if payload is not None else "GET",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        with contextlib.suppress(Exception):
            body = e.read().decode("utf-8", "replace")[:300]
        if e.code in (401, 403):
            raise ProviderUnavailableError(
                f"tabpfn-hosted rejected the request with HTTP {e.code}: the TABPFN_API_KEY "
                f"is missing permissions or invalid ({body})"
            ) from e
        if e.code == 429:
            reset = e.headers.get("x-ratelimit-reset") or e.headers.get("retry-after") or "unknown"
            raise ProviderUnavailableError(
                f"tabpfn-hosted quota exhausted (HTTP 429); resets at {reset}. "
                "Fail-open: retry after the reset window or use another backend."
            ) from e
        raise WorkerError(f"tabpfn-hosted predict/fit call failed with HTTP {e.code}: {body}") from e
    except (urllib.error.URLError, OSError, TimeoutError) as e:
        raise WorkerError(f"tabpfn-hosted unreachable: {e}") from e
    except json.JSONDecodeError as e:
        raise WorkerError("tabpfn-hosted returned unparseable JSON") from e


def _put_signed(url: str, body: bytes, required_headers: dict[str, str], timeout: float) -> None:
    req = urllib.request.Request(url, data=body, headers=dict(required_headers), method="PUT")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            if not 200 <= resp.status < 300:
                raise WorkerError(f"signed upload returned HTTP {resp.status}")
    except urllib.error.HTTPError as e:
        raise WorkerError(f"signed upload failed with HTTP {e.code}") from e
    except (urllib.error.URLError, OSError, TimeoutError) as e:
        raise WorkerError(f"signed upload unreachable: {e}") from e


def probe_detail() -> tuple[bool, str]:
    """(available, reason) via GET /tabpfn/get_model_limits — cheap authed GET, cached."""
    global _probe_cache
    if api_key() is None:
        return False, "not configured — set TABPFN_API_KEY"
    now = time.monotonic()
    if _probe_cache and now - _probe_cache[0] < PROBE_TTL_S:
        return _probe_cache[1]
    try:
        payload = _authed_json("/tabpfn/get_model_limits", None, PROBE_TIMEOUT_S)
        version = payload.get("default_model_version", "unknown")
        max_classes = (payload.get("max_model_limit") or {}).get("max_classes", "?")
        result = (True, f"prior labs api ok — default model {version} (max {max_classes} classes)")
    except ProviderUnavailableError as e:
        result = (False, str(e)[:200])
    except WorkerError as e:
        result = (False, str(e)[:200])
    _probe_cache = (now, result)
    return result


def is_available() -> bool:
    return probe_detail()[0]


def reset_probe_cache() -> None:
    global _probe_cache
    _probe_cache = None


# ── wire flow ─────────────────────────────────────────────────────────────────


def _columns(rows: list[dict], exclude: set[str]) -> list[str]:
    """First-seen column order across rows, minus excluded columns."""
    seen: list[str] = []
    for row in rows:
        for key in row:
            if key not in exclude and key not in seen:
                seen.append(key)
    return seen


def _to_csv_bytes(rows: list[dict], columns: list[str]) -> bytes:
    buf = io.StringIO()
    writer = csv.DictWriter(buf, fieldnames=columns, extrasaction="ignore")
    writer.writeheader()
    writer.writerows({c: row.get(c) for c in columns} for row in rows)
    return buf.getvalue().encode("utf-8")


def _upload_side(
    info: dict[str, Any], body: bytes, timeout: float, what: str, telemetry: dict[str, Any]
) -> None:
    urls = info.get("signed_urls") or []
    headers = info.get("required_headers") or {}
    if not urls:
        raise WorkerError(f"tabpfn-hosted returned no signed URL for {what}")
    t0 = time.monotonic()
    _put_signed(urls[0], body, headers, timeout)
    telemetry[f"{what}_upload_ms"] = int((time.monotonic() - t0) * 1000)


def run(
    rows_context: list[dict],
    rows_query: list[dict],
    task: TableTask,
    telemetry: dict[str, Any],
) -> list[Prediction]:
    key = api_key()
    if key is None:
        raise ProviderUnavailableError(
            "tabpfn-hosted backend is not configured: set TABPFN_API_KEY to your Prior Labs "
            "API key (https://ux.priorlabs.ai)."
        )
    timeout = float(_env("SDM1_TABPFN_TIMEOUT") or DEFAULT_TIMEOUT_S)

    fit_task, output_type = _WIRE_MAP[task.task_type]
    actual_column = None
    if task.task_type == "anomaly":
        actual_column = task.metadata.get("actual_column") or f"{task.target}_actual"
    feature_columns = _columns(rows_context + rows_query, {task.target, actual_column} - {None})

    x_train = _to_csv_bytes(rows_context, feature_columns)
    y_train = _to_csv_bytes([{task.target: r.get(task.target)} for r in rows_context], [task.target])
    x_test = _to_csv_bytes(rows_query, feature_columns)

    t0 = time.monotonic()
    prep = _authed_json(
        "/tabpfn/prepare_train_set_upload",
        {"x_train_info": {"format": "csv"}, "y_train_info": {"format": "csv"}},
        timeout,
    )
    _upload_side(prep.get("x_train_info") or {}, x_train, timeout, "x_train", telemetry)
    _upload_side(prep.get("y_train_info") or {}, y_train, timeout, "y_train", telemetry)

    fit = _authed_json(
        "/tabpfn/fit", {"train_set_upload_id": prep.get("train_set_upload_id"), "task": fit_task}, timeout
    )
    fitted_id = fit.get("fitted_train_set_id")
    if not fitted_id:
        raise WorkerError("tabpfn-hosted fit returned no fitted_train_set_id")

    tprep = _authed_json(
        "/tabpfn/prepare_test_set_upload",
        {"fitted_train_set_id": fitted_id, "x_test_info": {"format": "csv"}},
        timeout,
    )
    _upload_side(tprep.get("x_test_info") or {}, x_test, timeout, "x_test", telemetry)

    result = _authed_json(
        "/tabpfn/predict",
        {
            "test_set_upload_id": tprep.get("test_set_upload_id"),
            "fitted_train_set_id": fitted_id,
            "task_config": {"task": fit_task, "predict_params": {"output_type": output_type}},
        },
        timeout,
    )

    meta = result.get("metadata") or {}
    telemetry.update(
        {
            "billing_model_version": meta.get("billing_model_version"),
            "execution_mode": meta.get("execution_mode"),
            "cache_outcome": meta.get("cache_outcome"),
            "n_estimators": meta.get("n_estimators"),
            "tabpfn_task": fit_task,
            "output_type": output_type,
            "wire_latency_ms": int((time.monotonic() - t0) * 1000),
        }
    )

    predictions, structured_ok, error = parse_predict_response(
        result, task.task_type, rows_query, actual_column
    )
    telemetry["structured_ok"] = structured_ok
    if not structured_ok:
        telemetry["error_kind"] = "parse"
        raise WorkerError(f"tabpfn-hosted predict output unparseable: {error}")
    return predictions


register(
    Provider(
        id="tabpfn-hosted",
        name="Prior Labs TabPFN (hosted)",
        run=run,
        is_available=is_available,
        is_configured=is_configured,
        probe=probe_detail,
        description="Hosted tabular foundation models (classification/regression/forecast/anomaly) over the Prior Labs REST API — stdlib HTTP, no local stack.",
        tier="core",
        meta={
            "model_class": "hosted fit-per-call",
            "vendor": "Prior Labs",
            "auth_env": "TABPFN_API_KEY",
            "base_url": DEFAULT_BASE,
            "config_hint": "set TABPFN_API_KEY",
            "limits_note": "v3.5 default: 1M train rows, 160 classes, 20k cols; metered in data+prediction tokens (50M/day pool on current tier)",
        },
    )
)
