"""gen1 — the media generation lane of the sys1 house pattern.

sys1 judges what the work says · sdm1 scores what the work measures ·
sem1 indexes what the work looks like · **gen1 generates what the work needs
heard and seen.**

Three verbs, provider-abstracted: ``speak`` (text → speech), ``transcribe``
(audio → text), ``imagine`` (prompt → image). The wire fails over, the verdict
doesn't: ``speak`` walks qwen → stepfun → kokoro and names the provider that
answered. Generation is not calibrated — with one exception: transcription is
gradeable against ground truth, and joins the calibration loop naturally.
"""

from __future__ import annotations

# Importing this package registers every builtin provider (the side effect in
# gen1.providers). The order below is free: the provider modules reach the leaf
# modules as submodules, which the import system resolves while this package is
# still initializing.
from . import cascade, errors, keys, providers, registry, telemetry, types
from .cascade import imagine, speak, transcribe
from .doctor import doctor
from .errors import Gen1Error, ProviderCapabilityError, ProviderUnavailableError
from .probe import probe
from .registry import names as registry_names
from .types import (
    DoctorRow,
    Image,
    ImagineResult,
    ProviderProbe,
    SpeakResult,
    TelemetryRow,
    TranscriptResult,
)

__version__ = "0.1.0"

__all__ = [
    "DoctorRow",
    "Gen1Error",
    "Image",
    "ImagineResult",
    "ProviderCapabilityError",
    "ProviderProbe",
    "ProviderUnavailableError",
    "SpeakResult",
    "TelemetryRow",
    "TranscriptResult",
    "__version__",
    "cascade",
    "doctor",
    "errors",
    "imagine",
    "keys",
    "probe",
    "providers",
    "registry",
    "registry_names",
    "speak",
    "telemetry",
    "transcribe",
    "types",
]
