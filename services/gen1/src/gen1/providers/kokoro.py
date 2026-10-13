"""The Kokoro provider — the local floor of the speak cascade.

The floor's job is to *degrade*: when every remote leg refuses, kokoro still
answers. It runs as a subprocess (``_kokoro_runner.py``) inside an interpreter
that can import kokoro_onnx, so the heavy stack never enters the stdlib core.

Interpreter resolution, in order: ``GEN1_KOKORO_PYTHON`` → the env file
(``~/.config/gen1/env``) → ``sys.executable``. Model files resolve from
``GEN1_KOKORO_MODEL`` / ``GEN1_KOKORO_VOICES`` → ``~/.cache/gen1/tts`` →
``~/.cache/hyperframes/tts`` (this machine's existing copy). A missing
interpreter or model file is a refusal *by name* — with the download URL, so the
floor's degradation is always one command away from being repaired.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

from .. import telemetry
from ..duration import measure, wav_sample_rate
from ..errors import ProviderUnavailableError
from ..keys import load_key
from ..registry import ProviderSpec, register
from ..types import ImagineResult, ProviderProbe, SpeakResult, TranscriptResult

NAME = "kokoro"
RUNNER = Path(__file__).with_name("_kokoro_runner.py")
DEFAULT_VOICE = "af_heart"
CACHE = Path.home() / ".cache" / "gen1" / "tts"
HYPERFRAMES_CACHE = Path.home() / ".cache" / "hyperframes" / "tts"
DOWNLOAD_URLS = {
    "model": "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx",
    "voices": "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin",
}

spec = ProviderSpec(
    name=NAME,
    verbs=("speak",),
    key_env=None,
    extra="kokoro",
    local=True,
    note="local floor: kokoro-onnx in a subprocess; needs no key, degrades to nothing when the stack is absent",
)


def _interpreter() -> str:
    if os.environ.get("GEN1_KOKORO_PYTHON"):
        return os.environ["GEN1_KOKORO_PYTHON"]
    from_env_file = load_key("GEN1_KOKORO_PYTHON")
    if from_env_file:
        return from_env_file
    return sys.executable


def _model_files() -> tuple[str, str]:
    model = os.environ.get("GEN1_KOKORO_MODEL") or _existing(
        CACHE / "kokoro-v1.0.onnx", HYPERFRAMES_CACHE / "models" / "kokoro-v1.0.onnx"
    )
    voices = os.environ.get("GEN1_KOKORO_VOICES") or _existing(
        CACHE / "voices-v1.0.bin", HYPERFRAMES_CACHE / "voices" / "voices-v1.0.bin"
    )
    if not model or not Path(model).exists():
        raise ProviderUnavailableError(
            f"kokoro: model file missing (looked at $GEN1_KOKORO_MODEL, {CACHE}, {HYPERFRAMES_CACHE}) — "
            f"download it: {DOWNLOAD_URLS['model']}",
            submitted=False,
        )
    if not voices or not Path(voices).exists():
        raise ProviderUnavailableError(
            f"kokoro: voices file missing (looked at $GEN1_KOKORO_VOICES, {CACHE}, {HYPERFRAMES_CACHE}) — "
            f"download it: {DOWNLOAD_URLS['voices']}",
            submitted=False,
        )
    return model, voices


def _existing(*paths: Path) -> str | None:
    for path in paths:
        if path.exists():
            return str(path)
    return None


def _run(args: list[str], timeout: float = 300.0) -> subprocess.CompletedProcess:
    interpreter = _interpreter()
    return subprocess.run(  # the resolved interpreter runs our own runner
        [interpreter, str(RUNNER), *args], capture_output=True, text=True, timeout=timeout
    )


class _Kokoro:
    spec = spec

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
        model, voices = _model_files()
        import tempfile

        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as handle:
            out = handle.name
        try:
            proc = _run(
                [
                    "--text", text,
                    "--voice", voice or os.environ.get("GEN1_KOKORO_VOICE", DEFAULT_VOICE),
                    "--speed", str(speed if speed is not None else 1.0),
                    "--lang", _lang(language),
                    "--model", model,
                    "--voices-file", voices,
                    "--out", out,
                ]
            )
            if proc.returncode != 0:
                raise ProviderUnavailableError(
                    f"kokoro: the floor runner failed ({proc.returncode}) in {_interpreter()} — "
                    f"{(proc.stderr or proc.stdout).strip()[:400]}",
                    submitted=False,
                )
            audio = Path(out).read_bytes()
        finally:
            Path(out).unlink(missing_ok=True)
        duration = measure(audio, "wav")
        row = telemetry.build_row(
            payload=text, provider=NAME, model="kokoro-v1.0", duration_seconds=duration, request_id=None
        )
        telemetry.record(row)
        return SpeakResult(
            audio=audio,
            duration_seconds=duration,
            format="wav",
            sample_rate=wav_sample_rate(audio),
            model="kokoro-v1.0",
            provider=NAME,
            voice=voice or DEFAULT_VOICE,
            request_id=None,
            telemetry=row,
        )

    def transcribe(self, audio: bytes, *, language: str | None, model: str | None) -> TranscriptResult:
        raise ProviderUnavailableError("kokoro serves no ASR — transcribe walks the stepfun provider")

    def imagine(self, prompt: str, *, aspect_ratio: str, n: int) -> ImagineResult:
        raise ProviderUnavailableError("kokoro serves no image modality — imagine walks the wan provider")

    def probe(self) -> ProviderProbe:
        # The floor needs no key — its readiness is the interpreter and the model
        # files, so key_present is True (nothing is missing).
        key_present = True
        try:
            proc = _run(["--check"], timeout=60)
            report = json.loads(proc.stdout.strip() or "{}")
        except (OSError, subprocess.TimeoutExpired, json.JSONDecodeError) as exc:
            return ProviderProbe(
                provider=NAME,
                reachable=False,
                reachability_note=f"the floor runner could not run in {_interpreter()} — {exc}",
                voices=(DEFAULT_VOICE,),
                key_present=key_present,
                extra="kokoro",
            )
        if not report.get("ok"):
            return ProviderProbe(
                provider=NAME,
                reachable=False,
                reachability_note=str(report.get("error", "unknown floor failure")),
                voices=(DEFAULT_VOICE,),
                key_present=key_present,
                extra="kokoro",
            )
        voices = self._voice_catalog()
        return ProviderProbe(
            provider=NAME,
            reachable=True,
            reachability_note=f"kokoro_onnx {report.get('kokoro_onnx', '?')} in {_interpreter()}; "
            f"model {report.get('model')}",
            models=("kokoro-v1.0",),
            voices=voices,
            key_present=key_present,
            extra="kokoro",
        )

    def _voice_catalog(self) -> tuple[str, ...]:
        try:
            proc = _run(["--voices"], timeout=120)
            ids = json.loads(proc.stdout.strip() or "[]")
            return tuple(ids) if isinstance(ids, list) else (DEFAULT_VOICE,)
        except (OSError, subprocess.TimeoutExpired, json.JSONDecodeError):
            return (DEFAULT_VOICE,)


def _lang(language: str) -> str:
    """kokoro's lang codes are the American/British/Spanish/French/Italian/
    Hindi/Japanese/Chinese/Mandarin spellings; English maps to en-us."""
    return {"en": "en-us", "en-us": "en-us", "en-gb": "en-gb"}.get(language, language)


register(_Kokoro())
