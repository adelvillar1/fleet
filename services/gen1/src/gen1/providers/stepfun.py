"""The StepFun provider — stdlib only (urllib REST + SSE line parsing).

Both wires verified live 2026-10-08 against the plan host:
- TTS: POST /step_plan/v1/audio/speech → WAV bytes (verified 3.04s/24kHz).
- ASR: POST /step_plan/v1/audio/asr/sse → transcript.text.delta … transcript.text.done
  (round-tripped the pinned nar-s1 fixture back to its exact words).

The SSE event schema is pinned from the live stream: each event carries
``type``/``delta`` (deltas) or ``type``/``text`` (done). ASR normalizes
punctuation — the em-dash in the script's s1 line does not come back — so the
C2 regression compares word sequences.
"""

from __future__ import annotations

import base64
import os

from .. import telemetry
from ..duration import measure, wav_sample_rate
from ..errors import ProviderCapabilityError, ProviderUnavailableError
from ..keys import require_key
from ..registry import ProviderSpec, register
from ..transport import get_json, post_bytes, post_sse
from ..types import ImagineResult, ProviderProbe, SpeakResult, TranscriptResult

NAME = "stepfun"
DEFAULT_BASE = "https://api.stepfun.ai"
DEFAULT_TTS_MODEL = "stepaudio-2.5-tts"
DEFAULT_ASR_MODEL = "stepaudio-2.5-asr"
DEFAULT_VOICE = "lively-girl"
# The documented system-voice list (guides/developer/tts) — catalog knowledge,
# pinned, never fetched on the speak hot path (sem1's documented-not-discovered
# rule). The full list lives in TECH-DOC §3.
SYSTEM_VOICES = ("lively-girl",)
SERVED_MODELS = (DEFAULT_TTS_MODEL, "stepaudio-3-tts", DEFAULT_ASR_MODEL, "stepaudio-3-asr-max")
# The 1000-char input cap, from the provider's API reference.
INPUT_CHAR_LIMIT = 1000

spec = ProviderSpec(
    name=NAME,
    verbs=("speak", "transcribe"),
    key_env="STEPFUN_API_KEY",
    extra=None,
    local=False,
    note="REST TTS + JSON-SSE ASR on the step_plan host; stdlib only",
)


def _base() -> str:
    return os.environ.get("STEPFUN_BASE_URL", DEFAULT_BASE).rstrip("/")


def _auth() -> dict[str, str]:
    return {"Authorization": f"Bearer {require_key('STEPFUN_API_KEY')}"}


class _StepFun:
    spec = spec

    # -- speak ---------------------------------------------------------------

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
        if len(text) > INPUT_CHAR_LIMIT:
            raise ProviderCapabilityError(
                f"stepfun: input is {len(text)} chars, past the {INPUT_CHAR_LIMIT}-char cap of /audio/speech"
            )
        model = os.environ.get("STEPFUN_TTS_MODEL", DEFAULT_TTS_MODEL)
        payload: dict = {
            "model": model,
            "input": text,
            "voice": voice or os.environ.get("STEPFUN_TTS_VOICE", DEFAULT_VOICE),
            "language": language,
            "response_format": "wav",
        }
        if speed is not None:
            payload["speed"] = speed
        if instruction is not None:
            payload["instruction"] = instruction
        # The leg answers in its native container; the result names it.
        raw = self._post_audio(f"{_base()}/step_plan/v1/audio/speech", payload)
        duration = measure(raw, "wav")
        row = telemetry.build_row(
            payload=text, provider=NAME, model=model, duration_seconds=duration, request_id=None
        )
        telemetry.record(row)
        return SpeakResult(
            audio=raw,
            duration_seconds=duration,
            format="wav",
            sample_rate=wav_sample_rate(raw),
            model=model,
            provider=NAME,
            voice=payload["voice"],
            request_id=None,
            telemetry=row,
        )

    def _post_audio(self, url: str, payload: dict) -> bytes:
        return post_bytes(url, headers=_auth(), payload=payload, timeout=120)

    # -- transcribe ----------------------------------------------------------

    def transcribe(self, audio: bytes, *, language: str | None, model: str | None) -> TranscriptResult:
        model = model or os.environ.get("STEPFUN_ASR_MODEL", DEFAULT_ASR_MODEL)
        body = {
            "audio": {
                "data": base64.b64encode(audio).decode("ascii"),
                "input": {
                    "transcription": {"model": model, "language": language or "en"},
                    "format": {"type": _audio_format(audio)},
                },
            }
        }
        deltas: list[str] = []
        done_text: str | None = None
        session_id: str | None = None
        try:
            for event in post_sse(
                f"{_base()}/step_plan/v1/audio/asr/sse", headers=_auth(), payload=body, timeout=180
            ):
                kind = event.get("type", "")
                meta = event.get("meta") or {}
                session_id = session_id or (meta.get("session_id") if isinstance(meta, dict) else None)
                if kind == "transcript.text.delta" and isinstance(event.get("delta"), str):
                    deltas.append(event["delta"])
                elif kind == "transcript.text.done" and isinstance(event.get("text"), str):
                    done_text = event["text"]
        except Exception:  # a mid-stream failure was submitted
            telemetry.record(
                telemetry.build_row(
                    payload=audio, provider=NAME, model=model, duration_seconds=0.0, request_id=session_id
                )
            )
            raise
        text = done_text if done_text is not None else "".join(deltas)
        duration = measure(audio, _audio_format(audio))
        row = telemetry.build_row(
            payload=audio, provider=NAME, model=model, duration_seconds=duration, request_id=session_id
        )
        telemetry.record(row)
        return TranscriptResult(
            text=text,
            segments=tuple(deltas),
            duration_seconds=duration,
            model=model,
            provider=NAME,
            telemetry=row,
        )

    # -- imagine -------------------------------------------------------------

    def imagine(self, prompt: str, *, aspect_ratio: str, n: int) -> ImagineResult:
        raise ProviderCapabilityError("stepfun serves no image modality — imagine walks the wan provider")

    # -- probe ---------------------------------------------------------------

    def probe(self) -> ProviderProbe:
        note = "REST host reachable by TLS+HTTP; the key is proven by speak/transcribe (no cheap authenticated endpoint)"
        try:
            status, _ = get_json(_base(), timeout=15)
            reachable = 200 <= status < 500
        except ProviderUnavailableError as exc:
            return ProviderProbe(
                provider=NAME,
                reachable=False,
                reachability_note=str(exc),
                models=SERVED_MODELS,
                voices=SYSTEM_VOICES,
                key_present=bool(os.environ.get("STEPFUN_API_KEY")),
                extra=None,
            )
        return ProviderProbe(
            provider=NAME,
            reachable=reachable,
            reachability_note=note,
            models=SERVED_MODELS,
            voices=SYSTEM_VOICES,
            key_present=bool(os.environ.get("STEPFUN_API_KEY")),
            extra=None,
        )


def _audio_format(audio: bytes) -> str:
    if audio[:4] == b"RIFF" and audio[8:12] == b"WAVE":
        return "wav"
    if audio[:3] == b"ID3" or (len(audio) > 1 and audio[0] == 0xFF and (audio[1] & 0xE0) == 0xE0):
        return "mp3"
    return "mp3"


register(_StepFun())
