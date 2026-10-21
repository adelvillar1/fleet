"""Nace Perception through sys1.

sys1 is the provider lane: it already holds the Nace credentials
(`DREX_API_KEY`, loaded from `~/.config/sys1/env` or `SYS1_ENV_FILE`) and
the provider table that names them. The document parse runs here so the
key never reaches fleet's process, its settings, or the browser.

The flow is the vendor's: an upload grant, a multipart upload, a Parse
job, and a poll to a terminal state. The result is the vendor's Markdown
plus its page/unit counts and any `content_truncated` disclosure. No
retry is attempted: Parse bills when a job finishes, and a retry after an
ambiguous failure is a second bill.
"""

from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

DEFAULT_BASE = "https://console.nace.ai"
MAX_BYTES = 64 * 1024 * 1024
JOB_BUDGET_SECS = 280


def base_url() -> str:
    return (
        os.environ.get("SYS1_PERCEPTION_BASE")
        or os.environ.get("NACE_BASE_URL")
        or DEFAULT_BASE
    ).rstrip("/")


def api_key(cfg: dict) -> str | None:
    try:
        from .config import api_key_for
    except Exception:
        api_key_for = None
    key = api_key_for("drex", cfg) if api_key_for else None
    return key or os.environ.get("DREX_API_KEY") or os.environ.get("NACE_API_KEY")


def _request(method: str, url: str, key: str, body: bytes | None, timeout: float,
             headers: dict | None = None):
    request = urllib.request.Request(url, data=body, method=method)
    request.add_header("Authorization", "Bearer " + key)
    for name, value in (headers or {}).items():
        request.add_header(name, value)
    if body is not None and not (headers or {}).get("Content-Type"):
        request.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(request, timeout=timeout) as reply:
            raw = reply.read(4 * 1024 * 1024).decode("utf-8", "replace")
            return reply.status, json.loads(raw) if raw.strip() else {}
    except urllib.error.HTTPError as error:
        raw = error.read(1024 * 1024).decode("utf-8", "replace")
        try:
            payload = json.loads(raw) if raw.strip() else {}
        except json.JSONDecodeError:
            payload = {"raw": raw}
        return error.code, payload
    except (urllib.error.URLError, TimeoutError) as error:
        raise RuntimeError(f"the perception service did not answer: {error}") from error


def _error(payload: dict, status: int) -> tuple[str, str]:
    error = payload.get("error") if isinstance(payload, dict) else None
    if isinstance(error, dict):
        return str(error.get("code") or status), str(error.get("message") or "the request was refused")
    if isinstance(payload, dict) and payload.get("code"):
        return str(payload.get("code")), str(payload.get("message") or "the request was refused")
    return str(status), "the request was refused"


def _grant(key: str, name: str):
    status, payload = _request(
        "POST",
        base_url() + "/v1/documents/upload-grants",
        key,
        json.dumps({"path": name}).encode(),
        30,
    )
    if status >= 300:
        return None, _error(payload, status)
    grant = payload.get("result", payload) if isinstance(payload, dict) else {}
    workspace_id = grant.get("workspace_id")
    upload_url = grant.get("upload_url")
    token = grant.get("token")
    if not workspace_id or not upload_url or not token:
        return None, ("upload_grant", "the upload grant is missing its fields")
    return {"workspace_id": workspace_id, "upload_url": upload_url, "token": token}, None


def _upload(grant: dict, key: str, name: str, data: bytes):
    boundary = "fleet-sys1-" + str(abs(hash(data)))[:16]
    metadata = json.dumps({"path": name})
    body = b"".join([
        f"--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"; type=\"application/json\"\r\n\r\n".encode(),
        metadata.encode(),
        b"\r\n",
        f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n".encode(),
        data,
        f"\r\n--{boundary}--\r\n".encode(),
    ])
    url = grant["upload_url"]
    if not url.startswith("http"):
        url = base_url() + "/" + url.lstrip("/")
    status, payload = _request(
        "POST",
        url,
        key,
        body,
        120,
        {"X-Upload-Token": grant["token"],
         "Content-Type": f"multipart/form-data; boundary={boundary}"},
    )
    if status >= 300:
        return None, _error(payload, status)
    result = payload.get("result", payload) if isinstance(payload, dict) else {}
    file_id = None
    if isinstance(result, dict):
        file = result.get("file")
        if isinstance(file, dict):
            file_id = file.get("file_id")
        file_id = file_id or result.get("file_id")
    if not file_id:
        return None, ("upload", "the upload reply names no file id")
    return file_id, None


def _start_parse(grant: dict, key: str, file_id: str):
    body = json.dumps({
        "source": {
            "type": "workspace_file",
            "workspace_id": grant["workspace_id"],
            "file_id": file_id,
        },
        "output": {"formats": ["markdown"], "table_format": "markdown"},
        "parse_mode": "low",
    }).encode()
    status, payload = _request(
        "POST",
        base_url() + "/v1/documents/parse?wait_seconds=60",
        key,
        body,
        90,
    )
    if status >= 300:
        return None, None, _error(payload, status)
    if status == 200:
        return payload.get("result", payload), None, None
    job_id = payload.get("job_id") if isinstance(payload, dict) else None
    if not job_id:
        return None, None, ("parse", "the parse reply names no job id")
    return None, job_id, None


def _finish(result: dict, job_id: str | None) -> dict:
    result = result.get("result", result) if isinstance(result, dict) else {}
    document = result.get("document") or {}
    markdown = document.get("markdown") or document.get("text") or result.get("markdown") or ""
    if not markdown.strip():
        return {"ok": False, "code": "no_text", "reason": "the parse returned no text"}
    truncated = None
    for disclosure in result.get("disclosures") or []:
        if isinstance(disclosure, dict) and disclosure.get("code") == "content_truncated":
            truncated = {
                "original": disclosure.get("original_size") or 0,
                "retained": disclosure.get("retained_size") or 0,
            }
    return {
        "ok": True,
        "parser": "nace",
        "markdown": markdown,
        "pages": document.get("page_count"),
        "units": result.get("units"),
        "job_id": job_id or result.get("job_id") or "",
        "truncated": truncated,
    }


def _wait(job_id: str, key: str, job: dict | None, budget: float):
    deadline = time.monotonic() + budget
    pause = 1.0
    while True:
        if job is None:
            status, payload = _request(
                "GET", base_url() + f"/v1/documents/jobs/{job_id}", key, None, 10
            )
            if status >= 300:
                return None, _error(payload, status)
            job = payload
        state = job.get("status") if isinstance(job, dict) else None
        if state == "succeeded":
            result = job.get("result")
            if result is None:
                return None, ("job", f"job {job_id} succeeded with no result")
            if job.get("result_state") == "expired":
                return None, ("job", f"job {job_id} succeeded but its result expired")
            return result, None
        if state in ("failed", "cancelled"):
            error = job.get("error") or {}
            code = error.get("code") or state
            message = error.get("message") or "the job did not finish"
            return None, (str(code), f"job {job_id} {state}: {message}")
        if time.monotonic() + pause > deadline:
            return None, (
                "timeout",
                f"job {job_id} did not finish within {int(budget)}s; it may still complete in the vendor's console",
            )
        time.sleep(pause)
        job = None
        pause = min(pause * 2, 10.0)


def parse_document(path: str, cfg: dict, budget: float = JOB_BUDGET_SECS,
                   name: str | None = None) -> dict:
    """Parse one local document through Perception. Never raises for a
    document refusal; the caller gets a JSON-able dict."""
    source = Path(path)
    try:
        if not source.is_file():
            return {"ok": False, "code": "unreadable", "reason": f"{path} is not a file"}
        size = source.stat().st_size
        if size > MAX_BYTES:
            return {"ok": False, "code": "too_large",
                    "reason": f"{path} is larger than the {MAX_BYTES // (1024 * 1024)} MB bound"}
        data = source.read_bytes()
    except OSError as error:
        return {"ok": False, "code": "unreadable", "reason": f"{path}: {error}"}
    key = api_key(cfg)
    if not key:
        return {"ok": False, "code": "no_key",
                "reason": "sys1 has no DREX_API_KEY; set it in the sys1 provider env"}
    name = name or source.name
    try:
        grant, failure = _grant(key, name)
        if failure:
            return {"ok": False, "code": failure[0], "reason": failure[1]}
        file_id, failure = _upload(grant, key, name, data)
        if failure:
            return {"ok": False, "code": failure[0], "reason": failure[1]}
        result, job_id, failure = _start_parse(grant, key, file_id)
        if failure:
            return {"ok": False, "code": failure[0], "reason": failure[1]}
        if result is None:
            result, failure = _wait(job_id, key, None, budget)
            if failure:
                return {"ok": False, "code": failure[0], "reason": failure[1]}
        return _finish(result, job_id)
    except RuntimeError as error:
        return {"ok": False, "code": "transport", "reason": str(error)}
