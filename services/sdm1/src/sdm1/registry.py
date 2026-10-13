"""
Pluggable backend registry.

Backends register Provider instances here at import time; availability
probing happens through the provider (subprocess probes — torch is never
imported in-process, keeping the core stdlib-only).
"""

from __future__ import annotations

from .types import Provider

REGISTRY: dict[str, Provider] = {}


def register(provider: Provider) -> Provider:
    """Register (or replace) a backend provider by id."""
    REGISTRY[provider.id] = provider
    return provider


def get(provider_id: str) -> Provider:
    """Look up a provider; KeyError names the known ids."""
    try:
        return REGISTRY[provider_id]
    except KeyError:
        known = ", ".join(sorted(REGISTRY)) or "(none)"
        raise KeyError(f"unknown model provider {provider_id!r}; registered: {known}") from None


def all_ids() -> list[str]:
    return sorted(REGISTRY)
