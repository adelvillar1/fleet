"""Provider: st-worker — sentence-transformers via a venv subprocess.

The sdm1 nvidia-sdm pattern: the host spawns the venv's python running
worker_st.py, passes one JSON request on stdin, parses one JSON response.
One-shot per invocation (model load ~9 s), so callers batch the whole corpus.

Venv: <repo>/.venv-st by default (durable, in-repo, gitignored — NOT
/private/tmp, which macOS wipes). Override with SEM1_ST_VENV. Create it:
    uv venv .venv-st --python 3.12
    uv pip install --python .venv-st/bin/python sentence-transformers torchvision
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

from ..registry import register
from ..telemetry import Timed, record, sha256
from ..types import EmbedResult, ProviderUnavailableError

CAPABILITIES = {"text": True, "image": True, "audio": False, "video": False}
DEFAULT_MODEL = "google/embeddinggemma-2"
WORKER = Path(__file__).with_name("worker_st.py")


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def venv_python() -> Path:
    env = os.environ.get("SEM1_ST_VENV")
    if env:
        return Path(env) / "bin" / "python"
    return repo_root() / ".venv-st" / "bin" / "python"


def embed(items, *, model: str | None = None, timeout: float = 900.0, **_kw) -> EmbedResult:
    from ..types import normalize_items
    norm = normalize_items(items)
    mdl = model or DEFAULT_MODEL
    py = venv_python()
    if not py.exists():
        raise ProviderUnavailableError(
            f"st venv python not found at {py}. Create it:\n"
            f"  uv venv {py.parent.parent} --python 3.12\n"
            f"  uv pip install --python {py} sentence-transformers torchvision"
        )
    req = {"model": mdl, "items": norm}
    t = Timed()
    try:
        proc = subprocess.run(
            [str(py), str(WORKER), "--model", mdl],
            input=json.dumps(req).encode(),
            capture_output=True, timeout=timeout,
        )
    except subprocess.TimeoutExpired as e:
        raise ProviderUnavailableError(f"st-worker timed out after {timeout}s") from e
    except OSError as e:
        raise ProviderUnavailableError(f"st-worker spawn failed: {e}") from e
    ms = t.stop()

    try:
        resp = json.loads(proc.stdout.decode())
    except json.JSONDecodeError as e:
        raise ProviderUnavailableError(
            f"st-worker returned unparsable stdout ({proc.stderr.decode()[:200]})"
        ) from e
    if not resp.get("ok"):
        raise ProviderUnavailableError(f"st-worker error: {resp.get('error')}")

    vectors = resp["vectors"]
    dims = resp.get("dims") or (len(vectors[0]) if vectors else 0)
    record({
        "op": "embed", "provider": "st-worker", "model": mdl,
        "input_sha256": [sha256(json.dumps(it, sort_keys=True)) for it in norm],
        "chars": sum(len(str(it.get("text", ""))) for it in norm),
        "dims": dims, "count": len(vectors), "latency_ms": round(ms, 1),
    })
    return EmbedResult(
        vectors=vectors, dims=dims, model=mdl, provider="st-worker",
        telemetry={"latency_ms": round(ms, 1)},
        extras={"venv_python": str(py)},
    )


register("st-worker", embed, capabilities=CAPABILITIES,
         description="sentence-transformers venv worker; text + images (model-card interleaved format)")
