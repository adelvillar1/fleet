"""probe() — what every provider can prove, without spending money.

A probe never calls a verb: it uses the cheap authenticated endpoints that exist
(compatible-mode /models on the token-plan host), pinned catalog knowledge
(documentation, never discovered — sem1's rule), and the local floor's own
runner check. ``refused`` carries the route-rejection matrix so nobody re-probes
dead routes. probe is fail-open: an unreachable provider is a row saying so,
never an exception.
"""

from __future__ import annotations

from . import (
    providers,  # noqa: F401 — registration
    registry,
)
from .types import ProviderProbe


def probe(name: str | None = None) -> list[ProviderProbe]:
    """One row per registered provider (or just ``name``), sorted by name."""
    targets = [name] if name else registry.names()
    rows: list[ProviderProbe] = []
    for provider_name in targets:
        provider = registry.get(provider_name)
        if provider is None:
            continue
        try:
            rows.append(provider.probe())
        except Exception as exc:  # probe reports, never raises
            rows.append(
                _failed_row(provider_name, exc)
            )
    return rows


def _failed_row(provider_name: str, exc: Exception) -> ProviderProbe:
    spec = getattr(registry.get(provider_name), "spec", None)
    return ProviderProbe(
        provider=provider_name,
        reachable=False,
        reachability_note=f"probe raised {type(exc).__name__}: {exc}",
        key_present=False,
        extra=getattr(spec, "extra", None),
    )
