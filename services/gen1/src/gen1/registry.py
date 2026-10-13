"""The provider registry. A provider is one registered module — adding one
touches no core file (C5 proves it by registering a stub at runtime and running
the real verbs through it). Unknown providers decline by name.
"""

from __future__ import annotations

from dataclasses import dataclass

from .errors import ProviderUnavailableError


@dataclass(frozen=True)
class ProviderSpec:
    name: str
    verbs: tuple[str, ...]
    key_env: str | None
    extra: str | None
    local: bool
    note: str = ""


_registry: dict[str, object] = {}


def register(provider) -> None:
    """Register a provider instance (anything with a .spec)."""
    spec = provider.spec
    if spec.name in _registry:
        raise ValueError(f"provider {spec.name!r} is already registered")
    _registry[spec.name] = provider


def get(name: str):
    return _registry.get(name)


def names() -> list[str]:
    return sorted(_registry)


def all() -> list:
    return [_registry[name] for name in names()]


def require(name: str):
    """The provider, or a refusal that names it and lists what is registered —
    unknown providers decline by name, never silently."""
    provider = _registry.get(name)
    if provider is None:
        known = ", ".join(names()) or "(none registered)"
        raise ProviderUnavailableError(f"unknown provider {name!r} — registered: {known}")
    return provider


def unregister(name: str) -> None:
    """Test seam: drop a provider (the built-ins re-register on import)."""
    _registry.pop(name, None)
