#!/usr/bin/env python3
"""The kokoro floor runner — a small program the kokoro provider invokes as a
subprocess.

Why a subprocess: kokoro-onnx needs onnxruntime and numpy, and the floor's job
is to *degrade*, not to pull heavy dependencies into the stdlib core. The
provider resolves an interpreter that can import kokoro_onnx and runs this file
inside it; the parent package never imports the stack.

Modes (exactly one):
  --check              verify the stack imports and the model files exist
  --voices             print the voice ids the model file carries (JSON list)
  --text T --out P     synthesize T into the WAV file P

WAV is written with the stdlib ``wave`` module from the float samples kokoro
returns (int16 conversion); the parent measures duration and sample rate back
from the written header, so no duration claim crosses the boundary.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import wave
from pathlib import Path

DEFAULT_MODEL = Path.home() / ".cache" / "hyperframes" / "tts" / "models" / "kokoro-v1.0.onnx"
DEFAULT_VOICES = Path.home() / ".cache" / "hyperframes" / "tts" / "voices" / "voices-v1.0.bin"
DOWNLOAD_URLS = {
    "model": "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx",
    "voices": "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin",
}
SAMPLE_RATE = 24000


def _resolve(explicit: str | None, env: str | None, default: Path) -> Path:
    if explicit:
        return Path(explicit)
    if env and os.environ.get(env):
        return Path(os.environ[env])
    local = Path.home() / ".cache" / "gen1" / "tts" / default.name
    if local.exists():
        return local
    return default


def _model_path(explicit: str | None) -> Path:
    return _resolve(explicit, "GEN1_KOKORO_MODEL", DEFAULT_MODEL)


def _voices_path(explicit: str | None) -> Path:
    return _resolve(explicit, "GEN1_KOKORO_VOICES", DEFAULT_VOICES)


def _kokoro_version() -> str:
    try:
        from importlib.metadata import version

        return version("kokoro-onnx")
    except Exception:  # the version is a nicety, never a failure
        return "unknown"


def _check(model: str | None, voices: str | None) -> dict:
    report: dict = {"ok": False}
    try:
        import kokoro_onnx

        if not hasattr(kokoro_onnx, "Kokoro"):
            report["error"] = "kokoro_onnx is importable but carries no Kokoro class — wrong package?"
            return report
    except Exception as exc:  # the report names the reason
        report["error"] = f"import kokoro_onnx failed in {sys.executable}: {type(exc).__name__}: {exc}"
        return report
    report["kokoro_onnx"] = _kokoro_version()
    model_path, voices_path = _model_path(model), _voices_path(voices)
    for label, path, url in (
        ("model", model_path, DOWNLOAD_URLS["model"]),
        ("voices", voices_path, DOWNLOAD_URLS["voices"]),
    ):
        if not path.exists():
            report["error"] = f"{label} file missing: {path} — download it: {url}"
            return report
    report["model"], report["voices"] = str(model_path), str(voices_path)
    report["ok"] = True
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description="kokoro floor runner")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--voices", action="store_true")
    parser.add_argument("--text")
    parser.add_argument("--voice", default="af_heart")
    parser.add_argument("--speed", type=float, default=1.0)
    parser.add_argument("--lang", default="en-us")
    parser.add_argument("--model")
    parser.add_argument("--voices-file")
    parser.add_argument("--out")
    args = parser.parse_args()

    if args.check:
        print(json.dumps(_check(args.model, args.voices_file)))
        return 0
    if args.voices:
        from kokoro_onnx import Kokoro

        kokoro = Kokoro(str(_model_path(args.model)), str(_voices_path(args.voices_file)))
        print(json.dumps(sorted(kokoro.get_voices())))
        return 0
    if args.text is None or args.out is None:
        parser.error("--text and --out are required to synthesize")
        return 2
    import numpy as np
    from kokoro_onnx import Kokoro

    kokoro = Kokoro(str(_model_path(args.model)), str(_voices_path(args.voices_file)))
    samples, rate = kokoro.create(args.text, voice=args.voice, speed=args.speed, lang=args.lang)
    frames = (np.asarray(samples, dtype=np.float32) * 32767.0).clip(-32768, 32767).astype("<i2")
    with wave.open(args.out, "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(int(rate))
        handle.writeframes(frames.tobytes())
    return 0


if __name__ == "__main__":
    sys.exit(main())
