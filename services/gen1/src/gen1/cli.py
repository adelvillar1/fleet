"""The gen1 CLI — every verb from the shell, JSON when asked.

    gen1 speak "text" [--voice V] [--language L] [--format F] [--provider P] [--out FILE]
    gen1 transcribe AUDIO [--language L] [--model M] [--provider P]
    gen1 imagine "prompt" [--aspect 16:9] [--n N] [--provider P] [--out-dir DIR]
    gen1 probe [--provider P] [--json]
    gen1 doctor [--json]

Exit codes: 0 on success, 1 on a refusal (every leg declined), 2 on a usage
error. ``--json`` prints the normalized result shape, not a human rendering —
same contract as the Python API.
"""

from __future__ import annotations

import argparse
import base64
import json
import sys
from pathlib import Path

from . import cascade
from .errors import Gen1Error


def main(argv: list[str] | None = None) -> int:
    parser = _parser()
    args = parser.parse_args(argv)
    handler = getattr(args, "handler", None)
    if handler is None:
        parser.print_help()
        return 2
    try:
        return handler(args)
    except Gen1Error as exc:
        print(f"gen1: {exc}", file=sys.stderr)
        return 1


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="gen1", description=__doc__)
    sub = parser.add_subparsers(dest="command")

    speak = sub.add_parser("speak", help="text → speech")
    speak.add_argument("text")
    speak.add_argument("--voice")
    speak.add_argument("--language", default="en")
    speak.add_argument("--format", dest="format", default=None)
    speak.add_argument("--provider")
    speak.add_argument("--speed", type=float)
    speak.add_argument("--out", help="write the audio here instead of stdout")
    speak.add_argument("--json", action="store_true", help="print the result shape (audio as base64)")
    speak.set_defaults(handler=_cmd_speak)

    transcribe = sub.add_parser("transcribe", help="audio → text")
    transcribe.add_argument("audio", help="path to an audio file")
    transcribe.add_argument("--language")
    transcribe.add_argument("--model")
    transcribe.add_argument("--provider")
    transcribe.add_argument("--json", action="store_true")
    transcribe.set_defaults(handler=_cmd_transcribe)

    imagine = sub.add_parser("imagine", help="prompt → image(s)")
    imagine.add_argument("prompt")
    imagine.add_argument("--aspect", dest="aspect_ratio", default="1:1")
    imagine.add_argument("--n", type=int, default=1)
    imagine.add_argument("--provider")
    imagine.add_argument("--out-dir", default=".")
    imagine.add_argument("--json", action="store_true")
    imagine.set_defaults(handler=_cmd_imagine)

    probe = sub.add_parser("probe", help="what each provider can prove")
    probe.add_argument("--provider")
    probe.add_argument("--json", action="store_true")
    probe.set_defaults(handler=_cmd_probe)

    doctor = sub.add_parser("doctor", help="the environment, one row per provider")
    doctor.add_argument("--json", action="store_true")
    doctor.set_defaults(handler=_cmd_doctor)
    return parser


def _cmd_speak(args) -> int:
    result = cascade.speak(
        args.text,
        voice=args.voice,
        language=args.language,
        format=args.format,
        provider=args.provider,
        speed=args.speed,
    )
    if args.out:
        Path(args.out).write_bytes(result.audio)
        print(f"{result.provider} · {result.model} · {result.duration_seconds:.2f}s · {args.out}")
    elif args.json:
        print(json.dumps(_speak_json(result), indent=2))
    else:
        sys.stdout.buffer.write(result.audio)
    return 0


def _cmd_transcribe(args) -> int:
    audio = Path(args.audio).read_bytes()
    result = cascade.transcribe(audio, language=args.language, model=args.model, provider=args.provider)
    if args.json:
        print(json.dumps(_transcript_json(result), indent=2))
    else:
        print(result.text)
    return 0


def _cmd_imagine(args) -> int:
    result = cascade.imagine(args.prompt, aspect_ratio=args.aspect_ratio, n=args.n, provider=args.provider)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    written: list[str] = []
    for index, image in enumerate(result.images, start=1):
        path = out_dir / f"gen1-image-{index:03d}.{_ext(image.mime)}"
        path.write_bytes(image.data)
        written.append(str(path))
    if args.json:
        print(json.dumps(_imagine_json(result, written), indent=2))
    else:
        for path in written:
            print(f"{result.provider} · {result.model} · {path}")
    return 0


def _cmd_probe(args) -> int:
    from .probe import probe

    rows = probe(args.provider)
    if args.json:
        print(json.dumps([_probe_json(row) for row in rows], indent=2))
        return 0
    for row in rows:
        state = "reachable" if row.reachable else "unreachable"
        print(f"{row.provider:<10} {state:<12} key={'yes' if row.key_present else 'no '}  {row.reachability_note}")
        if row.refused:
            for line in row.refused:
                print(f"{'':<10} refused: {line}")
        if row.voices:
            print(f"{'':<10} voices: {', '.join(row.voices[:6])}{' …' if len(row.voices) > 6 else ''}")
    return 0


def _cmd_doctor(args) -> int:
    from .doctor import doctor, env_summary

    rows = doctor()
    environment = env_summary()
    if args.json:
        print(
            json.dumps(
                {
                    "environment": environment,
                    "providers": [
                        {
                            "provider": row.provider,
                            "key_present": row.key_present,
                            "key_env": row.key_env,
                            "reachable": row.reachable,
                            "extra": row.extra,
                            "version": row.version,
                            "note": row.note,
                        }
                        for row in rows
                    ],
                },
                indent=2,
            )
        )
        return 0
    print(f"gen1 {environment['gen1_version']} · telemetry {environment['telemetry_sink']}")
    for row in rows:
        print(
            f"{row.provider:<10} key={'yes' if row.key_present else 'no '} "
            f"reach={'yes' if row.reachable else 'no '} extra={row.extra or '-'} version={row.version or '-'}"
        )
        print(f"{'':<10} {row.key_env} — {row.note}")
    return 0


def _speak_json(result) -> dict:
    return {
        "provider": result.provider,
        "model": result.model,
        "voice": result.voice,
        "format": result.format,
        "sample_rate": result.sample_rate,
        "duration_seconds": result.duration_seconds,
        "request_id": result.request_id,
        "audio_base64": base64.b64encode(result.audio).decode("ascii"),
        "telemetry": _telemetry_json(result.telemetry),
    }


def _transcript_json(result) -> dict:
    return {
        "provider": result.provider,
        "model": result.model,
        "text": result.text,
        "segments": list(result.segments),
        "duration_seconds": result.duration_seconds,
        "telemetry": _telemetry_json(result.telemetry),
    }


def _imagine_json(result, written: list[str]) -> dict:
    return {
        "provider": result.provider,
        "model": result.model,
        "files": written,
        "images": [
            {"w": image.w, "h": image.h, "mime": image.mime, "bytes": len(image.data)} for image in result.images
        ],
        "telemetry": _telemetry_json(result.telemetry),
    }


def _probe_json(row) -> dict:
    return {
        "provider": row.provider,
        "reachable": row.reachable,
        "reachability_note": row.reachability_note,
        "models": list(row.models),
        "voices": list(row.voices),
        "refused": list(row.refused),
        "key_present": row.key_present,
        "extra": row.extra,
    }


def _telemetry_json(row) -> dict:
    return {
        "input_sha256": row.input_sha256,
        "input_chars": row.input_chars,
        "input_bytes": row.input_bytes,
        "provider": row.provider,
        "model": row.model,
        "duration_seconds": row.duration_seconds,
        "request_id": row.request_id,
    }


def _ext(mime: str) -> str:
    return mime.split("/")[-1].replace("jpeg", "jpg") or "png"


if __name__ == "__main__":
    sys.exit(main())
