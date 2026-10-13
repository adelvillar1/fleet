"""The provider protocol every provider module implements. The verbs take the
normalized arguments and return the normalized results; the probe reports what
the provider can prove. Refusals are Gen1Error subclasses raised by name.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from ..types import ImagineResult, ProviderProbe, SpeakResult, TranscriptResult


@runtime_checkable
class Provider(Protocol):
    spec: object

    def speak(
        self,
        text: str,
        *,
        voice: str | None,
        language: str,
        format: str | None,
        speed: float | None = None,
        instruction: str | None = None,
    ) -> SpeakResult: ...

    def transcribe(self, audio: bytes, *, language: str | None, model: str | None) -> TranscriptResult: ...

    def imagine(self, prompt: str, *, aspect_ratio: str, n: int) -> ImagineResult: ...

    def probe(self) -> ProviderProbe: ...
