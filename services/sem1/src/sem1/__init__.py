"""sem1 — the semantic embeddings lane of the sys1 house pattern.

Embeddings propose, sys1/sdm1 dispose: this library emits geometry (vectors,
cosine, top-k), never verdicts. Providers serve only the modalities they can
prove; out-of-capability requests raise instead of answering.

Public API:
    embed(items, provider="llama-server", **kw) -> EmbedResult
    probe(provider, endpoint=None, battery=False) -> dict
    similarity.cosine(a, b), similarity.top_k(query, vectors, k)
    store.VectorStore — packed float32 store + manifest
    errors: Sem1Error, ProviderUnavailableError, ProviderCapabilityError
"""

from __future__ import annotations

from . import registry, similarity  # noqa: F401
from .registry import all_ids, capabilities, enforce, get, register  # noqa: F401
from .types import (  # noqa: F401
    EmbedResult,
    ProviderCapabilityError,
    ProviderUnavailableError,
    Sem1Error,
    normalize_items,
)

__version__ = "0.1.0"

# Importing a provider module registers it (stdlib-only modules only — the
# st-worker host side is subprocess-based and stdlib too).
from .providers import http_llama, st_worker  # noqa: F401,E402


def embed(items, provider: str = "llama-server", **kw) -> EmbedResult:
    """Embed items through a registered provider.

    items: str (text) or dict — {"text": "..."} or {"image": "/path.png"}.
    The capability gate runs BEFORE any network or model work.
    """
    mod = registry.get(provider)
    norm = normalize_items(items)
    enforce(provider, norm)
    return mod["factory"](norm, **kw)


def probe(provider: str, endpoint: str | None = None, battery: bool = False, **kw) -> dict:
    from .probe import run_probe
    return run_probe(provider, endpoint=endpoint, battery=battery, **kw)
