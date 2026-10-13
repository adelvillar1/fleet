"""Capability probe — the component this library exists around.

`probe()` reports what an endpoint genuinely serves:
  - reachability + dims (cheap text embed)
  - declared capabilities (registry, not server marketing)
  - opt-in semantic battery: the 2026-10-07 mirror test. For a provider
    declaring image capability, solid red/blue squares and a red->blue
    gradient must embed so that each image is closer to its own description
    from BOTH directions and the gradient prefers the gradient text. This is
    the exact test the llama-server /v1/embeddings endpoint fails (it embeds
    base64 as text: direction flips between runs, distinct images land at
    0.998) and the sentence-transformers path passes.

A provider may not DECLARE image capability in the registry unless this
battery passes against it live. The probe is how that stays true.
"""

from __future__ import annotations

import json
import struct
import tempfile
import zlib
from pathlib import Path

from . import registry
from .similarity import cosine
from .types import ProviderUnavailableError, normalize_items


def _png(path: Path, w: int, h: int, pix) -> Path:
    def chunk(t: bytes, d: bytes) -> bytes:
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)
    raw = b"".join(b"\x00" + b"".join(bytes(pix(x, y)) for x in range(w)) for y in range(h))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
                     + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))
    return path


def mirror_battery(provider: str, workdir: Path | None = None) -> dict:
    """The 2026-10-07 mirror test. Returns {pass, checks: {...}, detail}."""
    mod = registry.get(provider)
    factory = mod["factory"]
    workdir = Path(workdir) if workdir else Path(tempfile.mkdtemp(prefix="sem1-battery-"))
    workdir.mkdir(parents=True, exist_ok=True)
    red = _png(workdir / "red.png", 64, 64, lambda x, y: (255, 0, 0))
    blue = _png(workdir / "blue.png", 64, 64, lambda x, y: (0, 0, 255))
    grad = _png(workdir / "grad.png", 64, 64, lambda x, y: (255, int(255 * y / 63), 0))

    def vec(item) -> list[float]:
        return factory([item]).vectors[0]

    i_red = vec({"image": str(red)})
    i_blue = vec({"image": str(blue)})
    i_grad = vec({"image": str(grad)})
    texts = ["a solid red square", "a solid blue square", "a gradient from red to blue"]
    tt = factory([{"text": t} for t in texts]).vectors

    checks = {
        "red_prefers_red": cosine(i_red, tt[0]) > cosine(i_red, tt[1]),
        "blue_prefers_blue": cosine(i_blue, tt[1]) > cosine(i_blue, tt[0]),
        "grad_prefers_grad": cosine(i_grad, tt[2]) > cosine(i_grad, tt[0]),
        "images_distinguishable": abs(cosine(i_red, i_blue)) < 0.97,
    }
    scores = {
        "img_red_x_text_red": round(cosine(i_red, tt[0]), 4),
        "img_red_x_text_blue": round(cosine(i_red, tt[1]), 4),
        "img_blue_x_text_blue": round(cosine(i_blue, tt[1]), 4),
        "img_blue_x_text_red": round(cosine(i_blue, tt[0]), 4),
        "img_grad_x_text_grad": round(cosine(i_grad, tt[2]), 4),
        "img_grad_x_text_red": round(cosine(i_grad, tt[0]), 4),
        "img_red_x_img_blue": round(cosine(i_red, i_blue), 4),
    }
    return {"pass": all(checks.values()), "checks": checks, "scores": scores, "workdir": str(workdir)}


def run_probe(provider: str, endpoint: str | None = None, battery: bool = False,
              model: str | None = None, **kw) -> dict:
    mod = registry.get(provider)
    caps = mod["capabilities"]
    out: dict = {"provider": provider, "declared_capabilities": caps, "endpoint": endpoint}
    text = "probe: sem1 capability check"
    try:
        kwargs = dict(kw)
        if endpoint:
            kwargs["endpoint"] = endpoint
        if model:
            kwargs["model"] = model
        r = mod["factory"]([{"text": text}], **kwargs)
        out.update({"reachable": True, "dims": r.dims, "model": r.model,
                    "latency_ms": r.telemetry.get("latency_ms")})
    except ProviderUnavailableError as e:
        out.update({"reachable": False, "error": str(e)[:300]})
        return out

    if battery:
        if caps.get("image"):
            try:
                kwargs = dict(kw)
                if endpoint:
                    kwargs["endpoint"] = endpoint
                if model:
                    kwargs["model"] = model
                out["battery"] = mirror_battery(provider)
            except ProviderUnavailableError as e:
                out["battery"] = {"pass": False, "error": str(e)[:300]}
        else:
            out["battery"] = {
                "pass": None, "skipped": "provider declares text-only; nothing to prove",
            }
    return out
