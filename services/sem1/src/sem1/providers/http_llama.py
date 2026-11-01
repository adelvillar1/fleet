"""Provider: native llama-server (or any OpenAI-style /v1/embeddings endpoint).

Text-only by declaration. On 2026-10-07 the local build (llama.cpp b11443)
served text at 768 dims / 46-86 ms but tokenized image data-URIs as text —
returning confident garbage with no error — so this provider refuses image
items at the door instead of trusting the server.

Endpoint: http://127.0.0.1:8901 (the LaunchAgent-owned native server). No API
key for localhost; pass api_key for hardened endpoints.
"""

from __future__ import annotations

import json
import socket
import time
import urllib.error
import urllib.request

from ..registry import register
from ..telemetry import Timed, record, sha256
from ..types import EmbedResult, ProviderUnavailableError

CAPABILITIES = {"text": True, "image": False, "audio": False, "video": False}
DEFAULT_ENDPOINT = "http://127.0.0.1:8901"
DEFAULT_MODEL = "embeddinggemma-2-BF16"

# The bounded busy-retry (KI-346/KI-325): a timeout means the endpoint
# is UP and holds the one slot for someone else, so a later attempt is
# worth making — twice, with rising backoff. An absent endpoint is
# never retried: starting it is the caller's answer, not time's.
RETRIES = 2
BACKOFF_SECONDS = (1.0, 3.0)


class ProviderBusyError(ProviderUnavailableError):
    """The endpoint is UP but BUSY: it held an answer past the budget
    because a concurrent embed owns the one slot (KI-346). A subclass
    so every caller's `except ProviderUnavailableError` still catches
    it, distinct so the retry — and only the retry — keys on it."""


def _busy(url: str, timeout: float) -> str:
    return (
        f"llama-server {url} did not answer within {timeout}s — it is up "
        f"but busy (a concurrent embed is in flight); retry"
    )


def _absent(url: str, error: object) -> str:
    return (
        f"llama-server endpoint unreachable at {url}: {error}. "
        f"Start it: launchctl kickstart gui/$(id -u)/com.adelvillar1.sem1-llama"
    )


def _request(url: str, body: dict, api_key: str | None, timeout: float) -> dict:
    """One request, with the error taxonomy SPLIT BY CAUSE (KI-346).

    The old single arm folded `socket.timeout` (an OSError subclass)
    into "unreachable", so a live-but-queued server was reported absent
    with a Start-it line that told the operator to start a server with
    days of uptime (KI-354). A timeout — connect or read — is the UP
    and BUSY case and raises [`ProviderBusyError`]; a refusal or DNS
    failure is the ABSENT case and keeps the Start-it line.
    """
    req = urllib.request.Request(url, method="POST")
    req.add_header("Content-Type", "application/json")
    if api_key:
        req.add_header("Authorization", f"Bearer {api_key}")
    try:
        with urllib.request.urlopen(req, json.dumps(body).encode(), timeout=timeout) as r:
            return json.loads(r.read())
    except urllib.error.HTTPError as e:
        detail = ""
        try:
            detail = e.read().decode()[:200]
        except Exception:
            pass
        raise ProviderUnavailableError(f"llama-server {url} -> HTTP {e.code}: {detail}") from e
    except urllib.error.URLError as e:
        # A refusal or DNS failure rides here with its cause; a
        # connect timeout rides here too, wrapped as the reason.
        reason = getattr(e, "reason", None)
        if isinstance(reason, (TimeoutError, socket.timeout)):
            raise ProviderBusyError(_busy(url, timeout)) from e
        raise ProviderUnavailableError(_absent(url, e)) from e
    except (TimeoutError, socket.timeout) as e:
        # A read timeout: the socket connected and the server held the
        # answer past the budget — up, but busy.
        raise ProviderBusyError(_busy(url, timeout)) from e
    except OSError as e:
        # A refusal that arrived bare, not wrapped in a URLError.
        raise ProviderUnavailableError(_absent(url, e)) from e


def embed(items, *, model: str | None = None, endpoint: str | None = None,
          api_key: str | None = None, timeout: float = 60.0, **_kw) -> EmbedResult:
    from ..types import normalize_items
    norm = normalize_items(items)
    texts = []
    for it in norm:
        if "text" not in it:
            from ..types import ProviderCapabilityError
            raise ProviderCapabilityError(
                "llama-server provider is TEXT-ONLY by declaration: the local "
                "build's /v1/embeddings tokenizes non-text payloads as text and "
                "returns confident garbage. Use the st-worker provider for images."
            )
        texts.append(it["text"])

    url = (endpoint or DEFAULT_ENDPOINT).rstrip("/") + "/v1/embeddings"
    body = {"input": texts}
    if model:
        body["model"] = model
    t = Timed()
    attempt = 0
    while True:
        try:
            data = _request(url, body, api_key, timeout)
            break
        except ProviderBusyError:
            # KI-346/KI-325's remedy: a busy one-slot endpoint is
            # retried with bounded backoff instead of surfacing an
            # absence — saturation degrades to retry-later, never to
            # "start the server that is already running" (KI-354).
            if attempt >= RETRIES:
                raise
            time.sleep(BACKOFF_SECONDS[min(attempt, len(BACKOFF_SECONDS) - 1)])
            attempt += 1
    ms = t.stop()

    vectors = [item["embedding"] for item in data.get("data", [])]
    if len(vectors) != len(texts):
        raise ProviderUnavailableError(
            f"llama-server returned {len(vectors)} vectors for {len(texts)} inputs"
        )
    dims = len(vectors[0]) if vectors else 0
    mdl = data.get("model") or model or DEFAULT_MODEL
    usage = data.get("usage") or {}
    record({
        "op": "embed", "provider": "llama-server", "model": mdl,
        "input_sha256": [sha256(x) for x in texts], "chars": sum(len(x) for x in texts),
        "dims": dims, "count": len(vectors), "latency_ms": round(ms, 1),
        "prompt_tokens": usage.get("prompt_tokens"),
    })
    return EmbedResult(
        vectors=vectors, dims=dims, model=mdl, provider="llama-server",
        telemetry={"latency_ms": round(ms, 1), "prompt_tokens": usage.get("prompt_tokens")},
        extras={"endpoint": url},
    )


register("llama-server", embed, capabilities=CAPABILITIES,
         description="native llama-server, OpenAI-style /v1/embeddings, text-only by declaration")
