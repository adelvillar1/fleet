"""The cascade — the pinned failure semantics, in one place.

The money rule: **generation is not idempotent.** A connection-phase failure
(DNS, refused connect — ``submitted=False``) retries once, because the request
never left and re-issuing it is free. A failure after submission
(``submitted=True`` — timeout in flight, reset, mid-stream drop, empty audio with
a live request id) never re-issues: the leg was billed, it fails over, and the
attempt's ``request_id`` reaches the telemetry row for reconciliation.

``speak`` walks qwen → stepfun → kokoro and returns the first leg that answers
with bytes. ``transcribe`` and ``imagine`` walk their single legs. Every refusal
is collected by name, and exhaustion is itself a refusal naming every leg that
declined. An unknown provider declines by name.
"""

from __future__ import annotations

from . import registry
from .errors import Gen1Error, ProviderUnavailableError
from .types import ImagineResult, SpeakResult, TranscriptResult

SPEAK_ORDER = ("qwen", "stepfun", "kokoro")
TRANSCRIBE_ORDER = ("stepfun",)
IMAGINE_ORDER = ("wan",)


def speak(
    text: str,
    *,
    voice: str | None = None,
    language: str = "en",
    format: str | None = None,
    provider: str | None = None,
    speed: float | None = None,
    instruction: str | None = None,
) -> SpeakResult:
    """Text → speech. The wire fails over, the verdict doesn't: the result names
    the provider that answered."""
    order = (provider,) if provider else SPEAK_ORDER
    return _walk("speak", order, text, voice=voice, language=language, format=format, speed=speed,
                 instruction=instruction)


def transcribe(
    audio: bytes, *, language: str | None = None, model: str | None = None, provider: str | None = None
) -> TranscriptResult:
    """Audio → text. The gradeable exception to gen1's not-calibrated rule: the
    transcript joins the calibration loop."""
    order = (provider,) if provider else TRANSCRIBE_ORDER
    return _walk("transcribe", order, audio, language=language, model=model)


def imagine(
    prompt: str, *, aspect_ratio: str = "1:1", n: int = 1, provider: str | None = None
) -> ImagineResult:
    """Prompt → image(s)."""
    order = (provider,) if provider else IMAGINE_ORDER
    return _walk("imagine", order, prompt, aspect_ratio=aspect_ratio, n=n)


def _named(name: str, exc: Gen1Error) -> str:
    """A leg's refusal, attributed exactly once. Providers name themselves in
    their messages, so a message that already leads with the leg is not
    prefixed again — ``wan: wan: …`` is a wart, not a detail."""
    text = str(exc)
    return text if text.startswith(f"{name}: ") else f"{name}: {text}"


def _walk(verb: str, order: tuple[str, ...], payload, **kwargs):
    refusals: list[str] = []
    for name in order:
        provider = registry.get(name)
        if provider is None:
            refusals.append(f"{name!r}: unknown provider — registered: {', '.join(registry.names()) or 'none'}")
            continue
        try:
            result = _call(provider, verb, payload, **kwargs)
        except Gen1Error as exc:
            if not getattr(exc, "submitted", True):
                # Connection phase: the request never left. Re-issue once, free.
                try:
                    result = _call(provider, verb, payload, **kwargs)
                except Gen1Error as retry:
                    refusals.append(f"{_named(name, retry)} (after one retry)")
                    continue
            else:
                refusals.append(_named(name, exc))
                continue
        if verb == "speak" and not result.audio:
            refusals.append(f"{name}: empty audio")
            continue
        return result
    raise ProviderUnavailableError(f"every {verb} leg refused — " + " | ".join(refusals))


def _call(provider, verb: str, payload, **kwargs):
    if verb == "speak":
        return provider.speak(
            payload,
            voice=kwargs.get("voice"),
            language=kwargs.get("language", "en"),
            format=kwargs.get("format"),
            speed=kwargs.get("speed"),
            instruction=kwargs.get("instruction"),
        )
    if verb == "transcribe":
        return provider.transcribe(payload, language=kwargs.get("language"), model=kwargs.get("model"))
    return provider.imagine(payload, aspect_ratio=kwargs.get("aspect_ratio", "1:1"), n=kwargs.get("n", 1))
