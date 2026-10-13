"""doctor() — the environment, one row per provider, moli/sys1 row style.

Each row says whether the provider's key is present (and where it is expected),
whether the provider is reachable, which extra carries its heavy stack, and the
version of that stack when it is installed. doctor never calls a verb and never
raises: it is the "is this machine ready" report, and a machine that is not
ready must still get a readable answer.
"""

from __future__ import annotations

import importlib.metadata
import os

from . import keys, registry


def doctor() -> list:
    from .types import DoctorRow

    rows: list[DoctorRow] = []
    for provider in registry.all():
        spec = provider.spec
        key_env = spec.key_env or ""
        key_present = bool(keys.load_key(key_env)) if key_env else True
        note = spec.note
        reachable = False
        try:
            reachable = provider.probe().reachable
        except Exception as exc:  # doctor reports, never raises
            note = f"{note} (probe raised {type(exc).__name__})"
        rows.append(
            DoctorRow(
                provider=spec.name,
                key_present=key_present,
                key_env=key_env or "(none — the floor needs no key)",
                reachable=reachable,
                note=note,
                extra=spec.extra,
                version=_version(spec),
            )
        )
    return rows


# The heavy stack each extra installs (the extra name is not the distribution
# name — gen1[qwen] pulls dashscope).
EXTRA_DISTRIBUTION = {"qwen": "dashscope"}


def _version(spec) -> str | None:
    """The heavy stack's version when installed: dashscope for the qwen leg,
    kokoro-onnx as reported by the floor runner's own check."""
    distribution = EXTRA_DISTRIBUTION.get(spec.extra or "")
    if distribution:
        try:
            return importlib.metadata.version(distribution)
        except importlib.metadata.PackageNotFoundError:
            return None
    if spec.name == "kokoro":
        return _kokoro_version()
    return None


def _kokoro_version() -> str | None:
    import json
    import subprocess

    from .providers.kokoro import RUNNER, _interpreter

    try:
        proc = subprocess.run(  # the resolved interpreter runs our own runner
            [_interpreter(), str(RUNNER), "--check"], capture_output=True, text=True, timeout=60
        )
        report = json.loads(proc.stdout.strip() or "{}")
        return report.get("kokoro_onnx")
    except (OSError, subprocess.TimeoutExpired, json.JSONDecodeError):
        return None


def env_summary() -> dict:
    """The machine's gen1-relevant environment, redacted: presence, never values."""
    return {
        "env_file": str(keys.ENV_FILE),
        "env_file_exists": os.path.exists(keys.ENV_FILE),
        "telemetry_sink": str(_telemetry_sink()),
        "gen1_version": _gen1_version(),
    }


def _telemetry_sink():
    from . import telemetry

    return telemetry.sink_path()


def _gen1_version() -> str:
    from . import __version__

    return __version__
