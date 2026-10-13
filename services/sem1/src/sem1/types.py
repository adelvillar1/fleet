"""sem1 contract types: the normalized result shape and the error taxonomy.

Contract rule (house pattern): result keys are only ever ADDED. Renaming or
removing a key below is a breaking change and needs a plan.

Errors:
    Sem1Error                  — base
    ProviderUnavailableError   — endpoint/venv/model unreachable (fail-open upstream)
    ProviderCapabilityError    — the request needs a modality the provider cannot
                                 prove it serves. This error exists because on
                                 2026-10-07 two different local servers happily
                                 returned confident text-of-base64 vectors for
                                 image requests with no error. Silent answers to
                                 out-of-capability requests are the one failure
                                 this library exists to make impossible.
"""

from __future__ import annotations

from dataclasses import dataclass, field


class Sem1Error(Exception):
    pass


class ProviderUnavailableError(Sem1Error):
    pass


class ProviderCapabilityError(Sem1Error):
    pass


# The normalized embed contract. Every provider returns exactly this shape;
# providers add provider-specific detail under `extras` (never at top level).
RESULT_KEYS = (
    "vectors", "dims", "model", "provider", "count", "telemetry", "extras",
)


@dataclass
class EmbedResult:
    vectors: list[list[float]]
    dims: int
    model: str
    provider: str
    telemetry: dict = field(default_factory=dict)
    extras: dict = field(default_factory=dict)

    @property
    def count(self) -> int:
        return len(self.vectors)

    def to_dict(self) -> dict:
        return {
            "vectors": self.vectors,
            "dims": self.dims,
            "model": self.model,
            "provider": self.provider,
            "count": self.count,
            "telemetry": self.telemetry,
            "extras": self.extras,
        }


def normalize_items(items) -> list[dict]:
    """Accept str (text) or dict items; return normalized dicts.

    A dict item may carry at most one modality payload:
        {"text": "..."}                — text
        {"image": "/path/to.png"}      — image file
    Mixed text+image interleave ("... <|image|> ...", {"image": [...]}) is the
    st-worker provider's own extension, not part of the normalized contract.
    """
    out: list[dict] = []
    for it in items:
        if isinstance(it, str):
            out.append({"text": it})
        elif isinstance(it, dict):
            if "text" not in it and "image" not in it:
                raise Sem1Error(f"item needs 'text' or 'image': keys={sorted(it)}")
            out.append(dict(it))
        else:
            raise Sem1Error(f"unsupported item type: {type(it).__name__}")
    return out
