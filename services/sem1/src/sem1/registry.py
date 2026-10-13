"""Provider registry — same extension point as sys1/sdm1.

A provider registers a factory and the capabilities it can PROVE. Capabilities
are declarations the probe verifies, not marketing: the HTTP provider ships
text-only even when the underlying server claims vision, because on 2026-10-07
servers advertised modalities their embed endpoint silently refused.
"""

from __future__ import annotations

from .types import Sem1Error

_PROVIDERS: dict[str, dict] = {}


def register(name: str, factory, capabilities: dict | None = None, description: str = "") -> None:
    _PROVIDERS[name] = {
        "factory": factory,
        "capabilities": dict(capabilities or {"text": True}),
        "description": description,
    }


def get(name: str) -> dict:
    if name not in _PROVIDERS:
        raise Sem1Error(f"unknown provider: {name} (registered: {', '.join(sorted(_PROVIDERS))})")
    return _PROVIDERS[name]


def capabilities(name: str) -> dict:
    return dict(get(name)["capabilities"])


def all_ids() -> list[str]:
    return sorted(_PROVIDERS)


def enforce(name: str, items: list[dict]) -> None:
    """Raise ProviderCapabilityError if any item needs an unproven modality.

    Called BEFORE any network/model work: out-of-capability requests fail fast
    and loud, never with a confident wrong answer.
    """
    caps = capabilities(name)
    for it in items:
        for mod in ("image", "audio", "video"):
            if mod in it and not caps.get(mod):
                from .types import ProviderCapabilityError
                raise ProviderCapabilityError(
                    f"provider '{name}' cannot prove it serves {mod} inputs "
                    f"(capabilities: {caps}); refused rather than answered. "
                    f"Server-side claims are not verified by this refusal — "
                    f"run `sem1 probe --provider {name} --battery` to test for real."
                )
