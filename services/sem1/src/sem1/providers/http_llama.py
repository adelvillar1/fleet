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
import urllib.error
import urllib.request

from ..registry import register
from ..telemetry import Timed, record, sha256
from ..types import EmbedResult, ProviderUnavailableError

CAPABILITIES = {"text": True, "image": False, "audio": False, "video": False}
DEFAULT_ENDPOINT = "http://127.0.0.1:8901"
DEFAULT_MODEL = "embeddinggemma-2-BF16"


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
    req = urllib.request.Request(url, method="POST")
    req.add_header("Content-Type", "application/json")
    if api_key:
        req.add_header("Authorization", f"Bearer {api_key}")
    t = Timed()
    try:
        with urllib.request.urlopen(req, json.dumps(body).encode(), timeout=timeout) as r:
            data = json.loads(r.read())
    except urllib.error.HTTPError as e:
        detail = ""
        try:
            detail = e.read().decode()[:200]
        except Exception:
            pass
        raise ProviderUnavailableError(f"llama-server {url} -> HTTP {e.code}: {detail}") from e
    except (urllib.error.URLError, OSError) as e:
        raise ProviderUnavailableError(
            f"llama-server endpoint unreachable at {url}: {e}. "
            f"Start it: launchctl kickstart gui/$(id -u)/com.adelvillar1.sem1-llama"
        ) from e
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
