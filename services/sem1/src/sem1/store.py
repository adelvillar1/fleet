"""Packed-float32 vector store with a JSON manifest.

Layout (under a store root, default ~/.local/share/sem1/vectors/):
    <model-slug>/vectors.bin   — packed float32 rows, row i at offset i*dims*4
    <model-slug>/manifest.json — {"model","dims","count","provider","entries":[{key,sha256}...]}

`rebuild()` writes the full set from the given entries (dedup by key), so a
second rebuild over the same corpus is idempotent by construction — duplicates
cannot accumulate. Vectors roundtrip bit-exactly through array('f') bytes.
"""

from __future__ import annotations

import json
import os
import re
from array import array
from pathlib import Path

from .types import Sem1Error

DEFAULT_ROOT = Path(os.environ["SEM1_VECTORS_DIR"]) if os.environ.get("SEM1_VECTORS_DIR") else (
    Path.home() / ".local" / "share" / "sem1" / "vectors"
)


def _slug(model: str) -> str:
    s = re.sub(r"[^A-Za-z0-9._-]+", "-", model).strip("-")
    return s or "model"


class VectorStore:
    def __init__(self, root: Path | str | None = None):
        self.root = Path(root) if root else DEFAULT_ROOT

    def _dir(self, model: str) -> Path:
        return self.root / _slug(model)

    def rebuild(self, model: str, dims: int, provider: str,
                entries: list[tuple[str, str, list[float]]],
                meta: list[dict] | None = None) -> dict:
        """entries: [(key, sha256, vector)], optional parallel `meta` dicts
        (op, ts, ...) stored inside manifest entries. Replaces the model's
        store wholesale."""
        if dims <= 0:
            raise Sem1Error(f"rebuild: bad dims {dims}")
        seen: set[str] = set()
        clean_entries: list[tuple[str, str, list[float]]] = []
        clean_meta: list[dict] = []
        for i, (key, sha, vec) in enumerate(entries):
            if key in seen:
                continue
            if len(vec) != dims:
                raise Sem1Error(f"rebuild: entry {key} has {len(vec)} dims, expected {dims}")
            seen.add(key)
            clean_entries.append((key, sha, vec))
            clean_meta.append(dict(meta[i]) if meta else {})

        d = self._dir(model)
        d.mkdir(parents=True, exist_ok=True)
        bin_path = d / "vectors.bin"
        flat: array = array("f")
        for _key, _sha, vec in clean_entries:
            flat.extend(array("f", vec))
        tmp = bin_path.with_suffix(".tmp")
        with tmp.open("wb") as f:
            flat.tofile(f)
        tmp.replace(bin_path)

        manifest = {
            "model": model,
            "dims": dims,
            "provider": provider,
            "count": len(clean_entries),
            "entries": [{"key": k, "sha256": s, **m}
                        for (k, s, _v), m in zip(clean_entries, clean_meta)],
        }
        mpath = d / "manifest.json"
        mtmp = mpath.with_suffix(".tmp")
        mtmp.write_text(json.dumps(manifest, indent=1) + "\n")
        mtmp.replace(mpath)
        return manifest

    def load(self, model: str) -> tuple[dict | None, list[list[float]]]:
        """(manifest, vectors) — (None, []) when the store doesn't exist yet."""
        d = self._dir(model)
        mpath = d / "manifest.json"
        bin_path = d / "vectors.bin"
        if not mpath.exists() or not bin_path.exists():
            return None, []
        manifest = json.loads(mpath.read_text())
        dims = manifest["dims"]
        flat: array = array("f")
        with bin_path.open("rb") as f:
            flat.fromfile(f, manifest["count"] * dims)
        vectors = [flat[i * dims:(i + 1) * dims].tolist() for i in range(manifest["count"])]
        return manifest, vectors
