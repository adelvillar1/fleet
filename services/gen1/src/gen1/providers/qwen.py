"""The Qwen provider — DashScope websocket TTS on the token-plan host.

The wire (live-verified 2026-10-08): the DASHSCOPE_API_KEY is token-plan-scoped,
so the standard DashScope hosts 401 it and every HTTP speech route on the
token-plan host itself refuses (404 /audio/speech, 500 chat/completions
modalities, 400 native aigi/aigc). ``qwen-audio-3.0-tts-plus`` is
**websocket-only** — the SpeechSynthesizer protocol at
``/api-ws/v1/inference``. That route matrix is pinned here so nobody re-probes
dead routes; see ``REFUSED``.

The one cheap authenticated endpoint that works is compatible-mode
``GET /compatible-mode/v1/models`` (200, lists the served models), which is
what ``probe`` uses.
"""

from __future__ import annotations

import os

from .. import telemetry
from ..duration import measure
from ..errors import ProviderUnavailableError
from ..keys import require_key
from ..registry import ProviderSpec, register
from ..transport import get_json
from ..types import ImagineResult, ProviderProbe, SpeakResult, TranscriptResult

NAME = "qwen"
DEFAULT_WS_URL = "wss://token-plan.ap-southeast-1.maas.aliyuncs.com/api-ws/v1/inference"
DEFAULT_MODEL = "qwen-audio-3.0-tts-plus"
DEFAULT_VOICE = "longanhuan_v3.6"
DEFAULT_BASE = "https://token-plan.ap-southeast-1.maas.aliyuncs.com"
SYSTEM_VOICES = ("longanhuan_v3.6",)
REFUSED = (
    "speech is websocket-only (/api-ws/v1/inference) — every HTTP route below is a recorded rejection, not a discovery",
    "HTTP /audio/speech → 404 (not mounted)",
    "HTTP chat/completions modalities:[text,audio] → 500 UNKNOWN",
    "HTTP aigi/text2speech + aigc/multimodal-generation → 400 url error (model/endpoint mismatch)",
    "standard dashscope hosts → 401 (key is token-plan-scoped)",
    "qwen3-tts-flash → Model not exist on this host",
)
SERVED_MODELS = ("qwen-audio-3.0-tts-plus", "qwen-audio-3.0-realtime-plus")

spec = ProviderSpec(
    name=NAME,
    verbs=("speak",),
    key_env="DASHSCOPE_API_KEY",
    extra="qwen",
    local=False,
    note="websocket-only TTS via the dashscope SDK on the token-plan host",
)


def _base() -> str:
    return os.environ.get("QWEN_BASE_URL", DEFAULT_BASE).rstrip("/")


class _Qwen:
    spec = spec

    def _synthesizer(self, model: str, voice: str):
        try:
            import dashscope
            from dashscope.audio.tts_v2 import SpeechSynthesizer
        except ImportError as exc:
            raise ProviderUnavailableError(
                "qwen: the dashscope SDK is not installed — pip install 'gen1[qwen]'",
                submitted=False,
            ) from exc
        dashscope.api_key = require_key("DASHSCOPE_API_KEY")
        dashscope.base_websocket_api_url = os.environ.get("QWEN_TTS_WS_URL", DEFAULT_WS_URL)
        return SpeechSynthesizer(model=model, voice=voice)

    def speak(
        self,
        text: str,
        *,
        voice: str | None,
        language: str,
        format: str | None,
        speed: float | None = None,
        instruction: str | None = None,
    ) -> SpeakResult:
        model = os.environ.get("QWEN_TTS_MODEL", DEFAULT_MODEL)
        voice = voice or os.environ.get("QWEN_TTS_VOICE", DEFAULT_VOICE)
        synth = self._synthesizer(model, voice)
        try:
            audio = synth.call(text)
        except Exception as exc:  # the websocket refused; a mid-stream drop was submitted
            telemetry.record(
                telemetry.build_row(
                    payload=text,
                    provider=NAME,
                    model=model,
                    duration_seconds=0.0,
                    request_id=_last_request_id(synth),
                )
            )
            raise ProviderUnavailableError(
                f"qwen: websocket call failed — {type(exc).__name__}: {exc}", submitted=True
            ) from exc
        request_id = _last_request_id(synth)
        if not audio:
            # Empty audio with a live request id is the exact failure the money
            # rule covers: the leg was billed, the bytes never came back.
            telemetry.record(
                telemetry.build_row(
                    payload=text, provider=NAME, model=model, duration_seconds=0.0, request_id=request_id
                )
            )
            raise ProviderUnavailableError(
                f"qwen: empty audio (request {request_id})", submitted=True
            )
        duration = measure(audio, "mp3")
        row = telemetry.build_row(
            payload=text, provider=NAME, model=model, duration_seconds=duration, request_id=request_id
        )
        telemetry.record(row)
        return SpeakResult(
            audio=audio,
            duration_seconds=duration,
            format="mp3",
            sample_rate=_mp3_sample_rate(audio),
            model=model,
            provider=NAME,
            voice=voice,
            request_id=request_id,
            telemetry=row,
        )

    def transcribe(self, audio: bytes, *, language: str | None, model: str | None) -> TranscriptResult:
        raise ProviderUnavailableError(
            "qwen serves no ASR on this host (websocket TTS only) — transcribe walks the stepfun provider"
        )

    def imagine(self, prompt: str, *, aspect_ratio: str, n: int) -> ImagineResult:
        raise ProviderUnavailableError(
            "qwen's image modality is the wan provider (same key, different wire) — imagine walks wan"
        )

    def probe(self) -> ProviderProbe:
        key_present = bool(os.environ.get("DASHSCOPE_API_KEY"))
        try:
            status, parsed = get_json(
                f"{_base()}/compatible-mode/v1/models",
                headers={"Authorization": f"Bearer {require_key('DASHSCOPE_API_KEY')}"},
                timeout=15,
            )
        except ProviderUnavailableError as exc:
            return ProviderProbe(
                provider=NAME,
                reachable=False,
                reachability_note=str(exc),
                models=SERVED_MODELS,
                voices=SYSTEM_VOICES,
                refused=REFUSED,
                key_present=key_present,
                extra="qwen",
            )
        if status == 200 and isinstance(parsed, dict):
            models = _model_ids(parsed)
            note = "compatible-mode /models answered 200 — the served-model list; speech itself is websocket-only"
        else:
            models = SERVED_MODELS
            note = f"compatible-mode /models answered HTTP {status} (region or key?)"
        return ProviderProbe(
            provider=NAME,
            reachable=200 <= status < 300,
            reachability_note=note,
            models=models,
            voices=SYSTEM_VOICES,
            refused=REFUSED,
            key_present=key_present,
            extra="qwen",
        )


def _last_request_id(synth) -> str | None:
    getter = getattr(synth, "get_last_request_id", None)
    if getter is None:
        return None
    try:
        return getter()
    except Exception:  # the id is telemetry, never a failure
        return None


def _model_ids(parsed: dict) -> tuple[str, ...]:
    data = parsed.get("data")
    if not isinstance(data, list):
        return SERVED_MODELS
    ids = [m.get("id") for m in data if isinstance(m, dict) and isinstance(m.get("id"), str)]
    return tuple(ids) or SERVED_MODELS


def _mp3_sample_rate(data: bytes) -> int:
    """The first frame's sample rate, read from the frame header after any ID3v2
    tag (MPEG1 Layer III: 44100 for the verified qwen output)."""
    pos = 0
    if data[:3] == b"ID3" and len(data) >= 10:
        pos = 10 + ((data[6] & 0x7F) << 21 | (data[7] & 0x7F) << 14 | (data[8] & 0x7F) << 7 | (data[9] & 0x7F))
    while pos < len(data) - 4:
        if data[pos] == 0xFF and (data[pos + 1] & 0xE0) == 0xE0:
            version = (data[pos + 1] >> 3) & 0x03
            rates = {3: 44100, 2: 22050, 0: 11025}
            return rates.get(version, 0)
        pos += 1
    return 0


register(_Qwen())
