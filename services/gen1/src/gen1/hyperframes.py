"""The hyperframes seam — narration routed through gen1.

hyperframes' media-use audio engine speaks one dialect: a neutral
``audio_request.json`` of lines in, an ``audio_meta.json`` of files and measured
durations out. gen1's ``speak`` already produces exactly what that dialect
needs — ``SpeakResult`` carries audio bytes, a duration measured from those
bytes, and the provider that answered — so this module is an **adapter, not a
rewrite**. It reads the request, walks the cascade once per line, and writes the
engine's answer shape.

The provider declaration lives in the hyperframes project's own
``hyperframes.json`` under ``media.voice``, so the tool config names the route
and this module is only what executes it. A project without the block still
works: the cascade order is the same default the library uses.

Two honest limits, both named rather than papered over:

* **No word timestamps.** ``words`` comes back empty. gen1 measures duration
  from bytes; it does not align text to audio. Captions are a separate pass
  (``gen1 transcribe``, or the engine's own chain), not something this adapter
  invents.
* **Narration only.** BGM and SFX are not gen1's lane. They are declined by
  name, the same rule every unknown provider follows — never a silent empty
  track that reads as a successful run.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Mapping
from pathlib import Path
from typing import Any

from . import cascade
from .errors import Gen1Error

# The one provider this adapter serves. The media-use engine's own chain is
# closed (heygen.tts, kokoro.local); gen1 is reached through this module, which
# is why the project's hyperframes.json names a command and not a provider id.
PROVIDER = "gen1"

# Where each spoken line lands, relative to the hyperframes project. The engine
# writes assets/voice/<id>.wav; gen1 keeps the same directory but the extension
# of what the leg actually served, because format is part of the pinned result
# contract and a transcode would be a rewrite.
VOICE_SUBDIR = "assets/voice"


class SeamError(Gen1Error):
    """The seam refused: a request it cannot serve, or a project it cannot read."""


def _project_dir(value: str | None) -> Path:
    return Path(value).expanduser().resolve() if value else Path.cwd().resolve()


def load_voice_config(project_dir: Path) -> dict[str, Any]:
    """The ``media.voice`` block from the project's ``hyperframes.json``.

    A missing file or a missing block is not an error — the cascade's own order
    is the default — so this never raises for an ordinary project.

    ``provider`` in that block is the seam's identity ("narration routes through
    gen1"), not a cascade leg: it is validated here and never forwarded, because
    forwarding it would ask the cascade for a provider named "gen1" and get a
    refusal. Pinning one leg is ``leg``.
    """
    config_path = project_dir / "hyperframes.json"
    if not config_path.is_file():
        return {}
    try:
        config = json.loads(config_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as exc:
        raise SeamError(f"{config_path}: unreadable — {exc}") from exc
    if not isinstance(config, dict):
        raise SeamError(f"{config_path}: expected a JSON object")
    media = config.get("media")
    if not isinstance(media, dict):
        return {}
    voice = media.get("voice")
    if not isinstance(voice, dict):
        return {}
    declared = voice.get("provider", PROVIDER)
    if declared != PROVIDER:
        raise SeamError(
            f"{config_path}: media.voice.provider is {declared!r} — this adapter serves {PROVIDER!r} only"
        )
    return {key: value for key, value in voice.items() if key != "provider"}


def _declined(what: str) -> str:
    return f"{PROVIDER}: {what} is not gen1's lane — narration only (speak). Route it through the engine's own providers."


def _validate_request(request: Any) -> dict[str, Any]:
    if not isinstance(request, dict):
        raise SeamError("audio_request.json: expected a JSON object")
    lines = request.get("lines", [])
    if not isinstance(lines, list):
        raise SeamError("audio_request.json: 'lines' must be a list")
    for index, line in enumerate(lines):
        if not isinstance(line, dict) or "id" not in line:
            raise SeamError(f"audio_request.json: lines[{index}] needs an object with an 'id'")
    if request.get("bgm"):
        raise SeamError(_declined("bgm"))
    if request.get("sfx"):
        raise SeamError(_declined("sfx"))
    speed = request.get("speed", 1)
    try:
        speed = float(speed)
    except (TypeError, ValueError):
        raise SeamError(f"audio_request.json: 'speed' must be a number, got {speed!r}") from None
    if not 0 < speed <= 3:
        raise SeamError(f"audio_request.json: speed must be above 0 and at most 3, got {speed}")
    return request


def speak_lines(
    lines: list[Mapping[str, Any]],
    *,
    project_dir: Path,
    voice: str | None = None,
    language: str = "en",
    speed: float | None = None,
    provider: str | None = None,
) -> tuple[list[dict[str, Any]], list[str]]:
    """Walk the cascade once per line. Returns ``(voices, anomalies)``.

    A line whose text is empty is skipped with an anomaly rather than sent to a
    provider, and a line whose cascade refuses is omitted with the refusal
    named — the same attribution rule the cascade itself uses, so a partial run
    is never mistaken for a complete one.
    """
    config = load_voice_config(project_dir)
    voice = voice or config.get("voice")
    provider = provider or config.get("leg")
    language = config.get("lang", language)
    speed = speed if speed is not None else config.get("speed")

    voice_dir = project_dir / VOICE_SUBDIR
    voice_dir.mkdir(parents=True, exist_ok=True)

    voices: list[dict[str, Any]] = []
    anomalies: list[str] = []
    for line in lines:
        line_id = str(line.get("id"))
        text = str(line.get("text", "")).strip()
        if not text:
            anomalies.append(f"line {line_id}: empty text — skipped")
            continue
        line_voice = line.get("voice") or voice
        try:
            result = cascade.speak(
                text,
                voice=line_voice,
                language=language,
                speed=speed,
                provider=provider,
            )
        except Gen1Error as exc:
            anomalies.append(f"line {line_id}: TTS failed — omitted ({exc})")
            continue
        # The extension is what the leg served: qwen answers mp3, stepfun and
        # kokoro answer wav. No transcode, and the meta row says which.
        extension = f".{result.format.lstrip('.')}"
        relative = f"{VOICE_SUBDIR}/{line_id}{extension}"
        (project_dir / relative).write_bytes(result.audio)
        voices.append({
            "id": line_id,
            "path": relative,
            "duration_s": round(result.duration_seconds, 3),
            "words": [],
            # Additive keys: the engine ignores what it does not know, and a
            # consumer that cares which leg answered reads it here.
            "provider": result.provider,
            "format": result.format,
            "voice": result.voice,
            "request_id": result.request_id,
        })
    return voices, anomalies


def render_request(request: Any, project_dir: Path) -> dict[str, Any]:
    """The engine's answer shape for one ``audio_request.json``.

    ``voices`` and ``total_duration_s`` are what the composition reads. Sections
    this adapter does not serve are left exactly as an existing ``--out`` file
    had them, so a run that only touches tts never blanks a bgm row.
    """
    validated = _validate_request(request)
    lines = validated.get("lines", [])
    voices, anomalies = speak_lines(lines, project_dir=project_dir)
    meta = {
        "voices": voices,
        "total_duration_s": round(sum(v["duration_s"] for v in voices), 3),
        "tts_provider": PROVIDER,
        "voice_id": voices[0]["voice"] if voices else None,
    }
    if anomalies:
        meta["anomalies"] = anomalies
    return meta


def merge_meta(existing: Mapping[str, Any] | None, fresh: Mapping[str, Any]) -> dict[str, Any]:
    """Overlay the tts section on whatever the previous run left behind."""
    merged = dict(existing or {})
    merged.update(fresh)
    return merged


def _cmd_render(args: argparse.Namespace) -> int:
    project_dir = _project_dir(args.project)
    request_path = Path(args.request).expanduser()
    try:
        request = json.loads(request_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as exc:
        raise SeamError(f"{request_path}: unreadable — {exc}") from exc

    out_path = Path(args.out).expanduser()
    existing: dict[str, Any] = {}
    if out_path.is_file():
        try:
            existing = json.loads(out_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError) as exc:
            raise SeamError(f"{out_path}: unreadable merge base — {exc}") from exc

    meta = merge_meta(existing, render_request(request, project_dir))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    for voice in meta["voices"]:
        print(f"  voice {voice['id']}: {voice['path']} ({voice['duration_s']}s, {voice.get('provider')})", file=sys.stderr)
    for anomaly in meta.get("anomalies", []):
        print(f"  {anomaly}", file=sys.stderr)
    print(f"wrote {out_path} — {len(meta['voices'])} voice(s), {meta['total_duration_s']}s total", file=sys.stderr)
    return 0


# The provider entry, as it is merged into a hyperframes project's own config.
# `provider` is the seam's identity and is never forwarded to the cascade; `leg`
# pins one cascade leg and is optional; the rest forward to speak().
PROVIDER_ENTRY = {
    "provider": PROVIDER,
    "command": "python -m gen1.hyperframes render",
    "voice": None,
    "lang": "en",
    "speed": 1.0,
    "leg": None,
}


def install_entry(project_dir: Path, *, voice: str | None = None, leg: str | None = None) -> str:
    """Merge the provider entry into ``<project>/hyperframes.json``. Idempotent.

    Every other key in that file — ``paths``, ``registry``, ``media.autoProxy``,
    ``authoringSkill`` — is preserved exactly, and a second run writes the same
    bytes it wrote the first time. Returns a one-line verdict for the caller.
    """
    config_path = project_dir / "hyperframes.json"
    config: dict[str, Any] = {}
    if config_path.is_file():
        try:
            config = json.loads(config_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError) as exc:
            raise SeamError(f"{config_path}: unreadable — {exc}") from exc
        if not isinstance(config, dict):
            raise SeamError(f"{config_path}: expected a JSON object")

    entry = dict(PROVIDER_ENTRY)
    if voice is not None:
        entry["voice"] = voice
    if leg is not None:
        entry["leg"] = leg
    entry = {key: value for key, value in entry.items() if value is not None}

    media = config.get("media")
    if not isinstance(media, dict):
        media = {}
    existing_voice = media.get("voice")
    if isinstance(existing_voice, dict) and existing_voice.get("provider") not in (None, PROVIDER):
        raise SeamError(
            f"{config_path}: media.voice already names {existing_voice.get('provider')!r} — "
            "move it aside before installing the gen1 seam"
        )
    media["voice"] = {**existing_voice, **entry} if isinstance(existing_voice, dict) else entry
    config["media"] = media

    payload = json.dumps(config, indent=2) + "\n"
    if config_path.is_file() and config_path.read_text(encoding="utf-8") == payload:
        return f"{config_path}: already routes narration through gen1"
    config_path.write_text(payload, encoding="utf-8")
    return f"{config_path}: media.voice now routes narration through gen1"


def _cmd_install(args: argparse.Namespace) -> int:
    project_dir = _project_dir(args.project)
    if not project_dir.is_dir():
        raise SeamError(f"{project_dir}: not a directory — run this inside a hyperframes project")
    print(install_entry(project_dir, voice=args.voice, leg=args.leg), file=sys.stderr)
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="python -m gen1.hyperframes", description=__doc__)
    sub = parser.add_subparsers(dest="command")

    render = sub.add_parser("render", help="one audio_request.json → one audio_meta.json")
    render.add_argument("--request", required=True, help="path to the media-use audio_request.json")
    render.add_argument("--out", required=True, help="path to write audio_meta.json")
    render.add_argument("--project", help="the hyperframes project dir (default: cwd)")
    render.set_defaults(handler=_cmd_render)

    install = sub.add_parser("install", help="idempotently add the provider entry to hyperframes.json")
    install.add_argument("--project", help="the hyperframes project dir (default: cwd)")
    install.add_argument("--voice", help="pin a voice for every line unless a line overrides it")
    install.add_argument("--leg", help="pin one cascade leg (qwen | stepfun | kokoro)")
    install.set_defaults(handler=_cmd_install)
    return parser


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
        print(f"{PROVIDER}: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":  # pragma: no cover - the CLI entry, exercised end to end
    raise SystemExit(main())
