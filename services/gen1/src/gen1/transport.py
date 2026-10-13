"""The stdlib HTTP transport every remote provider shares, with the money
classification attached.

Generation is not idempotent, so every transport failure says whether the
request may have been submitted (it was billed — do not re-issue) or failed in
the connection phase (it never left — re-issuing is free). The cascade reads
that flag; see errors.ProviderUnavailableError.
"""

from __future__ import annotations

import contextlib
import json
import socket
import urllib.error
import urllib.request

from .errors import ProviderUnavailableError

DEFAULT_TIMEOUT = 120.0


# Failures that provably happen before any request byte leaves: DNS resolution
# and a refused connect. Everything else — resets, timeouts, mid-stream drops —
# is conservatively "submitted": a billed request may be in flight.
_CONNECTION_PHASE = (socket.gaierror, ConnectionRefusedError)


def _submitted(reason: object) -> bool:
    return not isinstance(reason, _CONNECTION_PHASE)


def _refused(exc: Exception, *, what: str, url: str) -> ProviderUnavailableError:
    if isinstance(exc, urllib.error.HTTPError):
        body = exc.read()[:2000].decode("utf-8", "replace")
        return ProviderUnavailableError(f"{what}: HTTP {exc.code} from {url} — {body}", submitted=True)
    if isinstance(exc, urllib.error.URLError):
        reason = exc.reason
        return ProviderUnavailableError(
            f"{what}: {type(reason).__name__} reaching {url} — {reason}", submitted=_submitted(reason)
        )
    if isinstance(exc, TimeoutError):
        return ProviderUnavailableError(f"{what}: timed out reaching {url}", submitted=True)
    return ProviderUnavailableError(f"{what}: {type(exc).__name__} — {exc}", submitted=True)


def post_json(url: str, *, headers: dict[str, str], payload: dict, timeout: float = DEFAULT_TIMEOUT) -> dict:
    """POST JSON, return the parsed JSON body. Non-2xx and transport failures
    raise ProviderUnavailableError with the provider's own detail preferred."""
    request = urllib.request.Request(
        url, data=json.dumps(payload).encode("utf-8"), headers={"Content-Type": "application/json", **headers}
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        detail = _error_detail(exc)
        raise ProviderUnavailableError(f"{url}: {detail}", submitted=True) from exc
    except Exception as exc:  # classified by _refused
        raise _refused(exc, what=f"POST {url}", url=url) from exc


def post_bytes(url: str, *, headers: dict[str, str], payload: dict, timeout: float = DEFAULT_TIMEOUT) -> bytes:
    """POST JSON, return the raw response bytes (audio legs answer bytes)."""
    request = urllib.request.Request(
        url, data=json.dumps(payload).encode("utf-8"), headers={"Content-Type": "application/json", **headers}
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return response.read()
    except urllib.error.HTTPError as exc:
        detail = _error_detail(exc)
        raise ProviderUnavailableError(f"{url}: {detail}", submitted=True) from exc
    except Exception as exc:  # classified by _refused
        raise _refused(exc, what=f"POST {url}", url=url) from exc


def get_bytes(url: str, *, headers: dict[str, str] | None = None, timeout: float = 60.0) -> tuple[bytes, dict]:
    """GET raw bytes, returning (body, headers). Used to fetch the signed image
    URLs the image route answers with; the content-type header names the mime."""
    request = urllib.request.Request(url, headers=headers or {})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return response.read(), dict(response.headers)
    except urllib.error.HTTPError as exc:
        detail = _error_detail(exc)
        raise ProviderUnavailableError(f"{url}: {detail}", submitted=True) from exc
    except Exception as exc:  # classified by _refused
        raise _refused(exc, what=f"GET {url}", url=url) from exc


def get_json(url: str, *, headers: dict[str, str] | None = None, timeout: float = 30.0) -> tuple[int, dict | None]:
    """GET, returning (status, parsed-or-None). Used by the cheap authenticated
    probe endpoints; a non-2xx is returned, not raised, so probe can report."""
    request = urllib.request.Request(url, headers=headers or {})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read().decode("utf-8", "replace")
            return response.status, (json.loads(body) if body.strip() else None)
    except urllib.error.HTTPError as exc:
        return exc.code, None
    except Exception as exc:  # probe reports reachability, never raises
        raise _refused(exc, what=f"GET {url}", url=url) from exc


def post_sse(url: str, *, headers: dict[str, str], payload: dict, timeout: float = DEFAULT_TIMEOUT):
    """POST JSON and yield parsed SSE data events (each ``data:`` line as a
    dict). The stream is consumed lazily so a mid-stream failure surfaces as a
    refusal with submitted=True."""
    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json", "Accept": "text/event-stream", **headers},
    )
    try:
        response = urllib.request.urlopen(request, timeout=timeout)
    except urllib.error.HTTPError as exc:
        detail = _error_detail(exc)
        raise ProviderUnavailableError(f"{url}: {detail}", submitted=True) from exc
    except Exception as exc:
        raise _refused(exc, what=f"POST {url}", url=url) from exc
    return _sse_events(response)


def _sse_events(response):
    with response:
        for raw in response:
            line = raw.decode("utf-8", "replace").strip()
            if not line.startswith("data:"):
                continue
            data = line[5:].strip()
            if not data or data == "[DONE]":
                continue
            try:
                event = json.loads(data)
            except json.JSONDecodeError:
                continue
            if isinstance(event, dict):
                yield event


def _error_detail(exc: urllib.error.HTTPError) -> str:
    """The provider's own words, verbatim: message → code → HTTP status — a
    region or key mistake must stay distinguishable from a bad request."""
    body = exc.read()[:2000].decode("utf-8", "replace")
    parsed = None
    with contextlib.suppress(json.JSONDecodeError, ValueError):
        parsed = json.loads(body)
    if isinstance(parsed, dict):
        for key in ("message", "code", "error"):
            value = parsed.get(key)
            if isinstance(value, str) and value:
                return value
            if isinstance(value, dict):
                inner = value.get("message") or value.get("code")
                if isinstance(inner, str) and inner:
                    return inner
    return f"HTTP {exc.code} — {body[:400]}" if body else f"HTTP {exc.code}"
