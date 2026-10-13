"""The Wan provider — the image leg, ported from design-canvas's solved wire.

The token-plan host's native multimodal-generation route (live-verified there
2026-08-08, re-verified here 2026-10-08): the same DASHSCOPE_API_KEY the qwen leg
uses, posted as a user message whose content is the prompt text. The response
carries signed OSS URLs; the bytes are fetched with a 60s timeout under an SSRF
guard — a result host outside aliyuncs.com is refused by name, never fetched.
No retries: a 4xx is the provider's answer, verbatim (message → code → HTTP
status), so a region or key mistake stays distinguishable from a bad request.
"""

from __future__ import annotations

import os
import urllib.parse

from .. import telemetry
from ..errors import ProviderCapabilityError, ProviderUnavailableError
from ..keys import require_key
from ..registry import ProviderSpec, register
from ..transport import get_bytes, get_json, post_json
from ..types import Image, ImagineResult, ProviderProbe, SpeakResult, TranscriptResult

NAME = "wan"
DEFAULT_BASE = "https://token-plan.ap-southeast-1.maas.aliyuncs.com"
DEFAULT_MODEL = "wan2.7-image-pro"
# The five aspect ratios the route accepts, as pixel pairs.
ASPECT_PIXELS = {
    "1:1": "1024*1024",
    "3:2": "1248*832",
    "16:9": "1440*810",
    "2:3": "832*1248",
    "9:16": "810*1440",
}
GENERATE_TIMEOUT = 150.0
FETCH_TIMEOUT = 60.0

spec = ProviderSpec(
    name=NAME,
    verbs=("imagine",),
    key_env="DASHSCOPE_API_KEY",
    extra=None,
    local=False,
    note="native multimodal-generation on the token-plan host; images fetched from signed OSS URLs",
)


def _base() -> str:
    return os.environ.get("DASHSCOPE_BASE_URL", DEFAULT_BASE).rstrip("/")


def _dims_from_usage(usage: object) -> tuple[int, int] | None:
    """usage.size arrives as "1488*704" — the actual pixels, which differ from
    the requested size; it is the truth when present."""
    if not isinstance(usage, dict):
        return None
    size = usage.get("size")
    if isinstance(size, str) and "*" in size:
        w, _, h = size.partition("*")
        if w.strip().isdigit() and h.strip().isdigit():
            return int(w), int(h)
    return None


def _requested_dims(aspect_ratio: str) -> tuple[int, int]:
    w, _, h = ASPECT_PIXELS[aspect_ratio].partition("*")
    return int(w), int(h)


class _Wan:
    spec = spec

    def imagine(self, prompt: str, *, aspect_ratio: str, n: int) -> ImagineResult:
        if aspect_ratio not in ASPECT_PIXELS:
            raise ProviderCapabilityError(
                f"wan: aspect ratio {aspect_ratio!r} is not served — pick from {', '.join(ASPECT_PIXELS)}"
            )
        n = max(1, min(4, n))
        model = os.environ.get("IMAGE_MODEL", DEFAULT_MODEL)
        payload = {
            "model": model,
            "input": {"messages": [{"role": "user", "content": [{"text": prompt}]}]},
            "parameters": {"size": ASPECT_PIXELS[aspect_ratio], "n": n, "watermark": False},
        }
        parsed = post_json(
            f"{_base()}/api/v1/services/aigc/multimodal-generation/generation",
            headers={"Authorization": f"Bearer {require_key('DASHSCOPE_API_KEY')}"},
            payload=payload,
            timeout=GENERATE_TIMEOUT,
        )
        images: list[Image] = []
        requested = _requested_dims(aspect_ratio)
        actual = _dims_from_usage(parsed.get("usage"))
        dims = actual or requested
        for choice in _choices(parsed):
            message = choice.get("message") if isinstance(choice, dict) else None
            if not isinstance(message, dict):
                continue
            for part in message.get("content") or []:
                if not isinstance(part, dict):
                    continue
                url = part.get("image")
                if not isinstance(url, str) or not url:
                    continue
                data, mime = self._fetch(url)
                images.append(Image(data=data, w=dims[0], h=dims[1], mime=mime))
        if not images:
            raise ProviderCapabilityError("wan: the route answered but carried no image")
        row = telemetry.build_row(
            payload=prompt, provider=NAME, model=model, duration_seconds=0.0, request_id=None
        )
        telemetry.record(row)
        return ImagineResult(images=tuple(images), model=model, provider=NAME, telemetry=row)

    def _fetch(self, url: str) -> tuple[bytes, str]:
        """Fetch a signed OSS URL under the SSRF guard: the host must be an
        aliyuncs.com host, or the response is refused by name. The mime comes
        from the content-type header (parameters stripped, default image/png)."""
        host = urllib.parse.urlparse(url).hostname or ""
        if not (host == "aliyuncs.com" or host.endswith(".aliyuncs.com")):
            raise ProviderCapabilityError(f"Unexpected image host: {host}")
        raw, headers = get_bytes(url, timeout=FETCH_TIMEOUT)
        content_type = (headers.get("Content-Type") or headers.get("content-type") or "image/png")
        mime = content_type.split(";")[0].strip() or "image/png"
        return raw, mime

    def speak(self, text, *, voice, language, format, speed=None, instruction=None) -> SpeakResult:
        raise ProviderUnavailableError("wan serves no speech — speak walks qwen → stepfun → kokoro")

    def transcribe(self, audio: bytes, *, language: str | None, model: str | None) -> TranscriptResult:
        raise ProviderUnavailableError("wan serves no ASR — transcribe walks the stepfun provider")

    def probe(self) -> ProviderProbe:
        try:
            status, parsed = get_models()
        except ProviderUnavailableError as exc:
            return ProviderProbe(
                provider=NAME,
                reachable=False,
                reachability_note=str(exc),
                models=(DEFAULT_MODEL,),
                refused=REFUSED,
                key_present=bool(os.environ.get("DASHSCOPE_API_KEY")),
                extra=None,
            )
        return ProviderProbe(
            provider=NAME,
            reachable=200 <= status < 300,
            reachability_note=(
                "compatible-mode /models answered — the image route itself is probed by imagine()"
                if 200 <= status < 300
                else f"compatible-mode /models answered HTTP {status} (region or key?)"
            ),
            models=image_models(parsed),
            refused=REFUSED,
            key_present=bool(os.environ.get("DASHSCOPE_API_KEY")),
            extra=None,
        )


REFUSED = (
    "HTTP /models (bare) → 404 on this host",
    "standard dashscope hosts → 401 (key is token-plan-scoped)",
)


def get_models() -> tuple[int, dict | None]:
    return get_json(
        f"{_base()}/compatible-mode/v1/models",
        headers={"Authorization": f"Bearer {require_key('DASHSCOPE_API_KEY')}"},
        timeout=15,
    )


def image_models(parsed: dict | None) -> tuple[str, ...]:
    data = (parsed or {}).get("data")
    if not isinstance(data, list):
        return (DEFAULT_MODEL,)
    ids = [
        m.get("id")
        for m in data
        if isinstance(m, dict) and isinstance(m.get("id"), str) and ("wan" in m["id"] or "image" in m["id"])
    ]
    return tuple(ids) or (DEFAULT_MODEL,)


def _choices(parsed: dict):
    output = parsed.get("output")
    if not isinstance(output, dict):
        return ()
    choices = output.get("choices")
    return choices if isinstance(choices, list) else ()


register(_Wan())
