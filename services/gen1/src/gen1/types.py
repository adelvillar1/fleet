"""The result contracts. One normalized shape per verb.

House rule: result keys are only ever added — renaming or removing one is a
breaking change. ``duration_seconds`` is measured from the returned bytes, never
the provider's claim, and ``provider`` is required: the wire fails over, the
verdict doesn't, so every result names the leg that answered.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TelemetryRow:
    """A structurally redacted usage row. It carries counts and identifiers,
    never content: no input text, no audio bytes, no key material."""

    input_sha256: str
    input_chars: int
    input_bytes: int
    provider: str
    model: str
    duration_seconds: float
    request_id: str | None


@dataclass(frozen=True)
class SpeakResult:
    audio: bytes
    duration_seconds: float
    format: str
    sample_rate: int
    model: str
    provider: str
    voice: str
    request_id: str | None
    telemetry: TelemetryRow


@dataclass(frozen=True)
class TranscriptResult:
    text: str
    segments: tuple[str, ...]
    duration_seconds: float
    model: str
    provider: str
    telemetry: TelemetryRow


@dataclass(frozen=True)
class Image:
    data: bytes
    w: int
    h: int
    mime: str


@dataclass(frozen=True)
class ImagineResult:
    images: tuple[Image, ...]
    model: str
    provider: str
    telemetry: TelemetryRow


@dataclass(frozen=True)
class ProviderProbe:
    """One provider's capability report. ``voices`` is pinned catalog knowledge
    (documented, not discovered — sem1's rule); ``models`` is served-model
    knowledge; ``refused`` is the route-rejection matrix, so nobody re-probes
    dead routes."""

    provider: str
    reachable: bool
    reachability_note: str
    models: tuple[str, ...] = ()
    voices: tuple[str, ...] = ()
    refused: tuple[str, ...] = ()
    key_present: bool = False
    extra: str | None = None


@dataclass(frozen=True)
class DoctorRow:
    """One doctor row: key presence, reachability, versions — the moli/sys1 row
    style."""

    provider: str
    key_present: bool
    key_env: str
    reachable: bool
    note: str
    extra: str | None = None
    version: str | None = None
