"""Key resolution. Keys come from the environment first, then the key file at
~/.config/gen1/env (the dev-decisions precedent: chmod 600, never committed).
The machine's existing key homes are recorded, not moved — an operator points
gen1 at them by exporting the variable or writing the key file."""

from __future__ import annotations

import os
from pathlib import Path

from .errors import ProviderUnavailableError

ENV_FILE = Path.home() / ".config" / "gen1" / "env"

# The environment variable each provider's key lives in.
KEY_ENV = {
    "qwen": "DASHSCOPE_API_KEY",
    "wan": "DASHSCOPE_API_KEY",
    "stepfun": "STEPFUN_API_KEY",
    "kokoro": None,  # the offline floor needs no key
}


def _read_env_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            name, _, value = line.partition("=")
            values[name.strip()] = value.strip().strip('"').strip("'")
    except OSError:
        return {}
    return values


def load_key(name: str) -> str | None:
    """Resolve a key by environment-variable name: process env, then the key
    file. Returns None when neither holds it."""
    value = os.environ.get(name)
    if value:
        return value.strip()
    return _read_env_file(ENV_FILE).get(name) or None


def require_key(env_name: str) -> str:
    """The key, or a refusal that names the variable and the key file — so the
    operator knows exactly where to put it."""
    key = load_key(env_name)
    if not key:
        raise ProviderUnavailableError(
            f"{env_name} not set — export it or add it to {ENV_FILE} (chmod 600, never committed)"
        )
    return key
