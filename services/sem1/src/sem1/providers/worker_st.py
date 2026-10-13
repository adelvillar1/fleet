"""Worker script — runs INSIDE the st venv (sentence-transformers + torchvision).

One JSON request on stdin:
    {"model": "google/embeddinggemma-2", "items": [{"text": "..."} | {"image": "/path.png"}]}
One JSON response on stdout:
    {"ok": true, "vectors": [[...]], "dims": N, "model": "..."}
or {"ok": false, "error": "..."} (exit 0 either way — the host parses, never guesses).

Image items follow the model card's interleaved format: {"text": "<|image|>",
"image": [path]}. Deliberately one-shot (load model, embed batch, exit) — the
sdm1 nvidia-sdm subprocess pattern; callers batch the whole corpus per call.
"""

from __future__ import annotations

import json
import sys


def main() -> int:
    try:
        req = json.loads(sys.stdin.read())
    except json.JSONDecodeError as e:
        json.dump({"ok": False, "error": f"bad request JSON: {e}"}, sys.stdout)
        return 0

    model_name = req.get("model", "google/embeddinggemma-2")
    items = req.get("items", [])

    try:
        from sentence_transformers import SentenceTransformer
    except Exception as e:  # ImportError and friends
        json.dump({"ok": False, "error": f"sentence-transformers unavailable in venv: {e}"}, sys.stdout)
        return 0

    try:
        model = SentenceTransformer(model_name)
        inputs: list = []
        for it in items:
            if "image" in it:
                inputs.append({"text": it.get("text", "<|image|>"), "image": [it["image"]]})
            else:
                inputs.append(it.get("text", ""))
        vecs = model.encode(inputs)
        vectors = [v if isinstance(v, list) else [float(x) for x in v] for v in vecs.tolist()]
        json.dump({
            "ok": True,
            "vectors": vectors,
            "dims": len(vectors[0]) if vectors else 0,
            "model": model_name,
        }, sys.stdout)
    except Exception as e:
        json.dump({"ok": False, "error": f"{type(e).__name__}: {e}"}, sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
