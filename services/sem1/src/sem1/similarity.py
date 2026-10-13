"""Pure similarity math: cosine and top-k. No I/O, no providers."""

from __future__ import annotations

import math


def cosine(a: list[float], b: list[float]) -> float:
    if len(a) != len(b):
        raise ValueError(f"cosine: length mismatch {len(a)} != {len(b)}")
    dot = 0.0
    na = 0.0
    nb = 0.0
    for x, y in zip(a, b):
        dot += x * y
        na += x * x
        nb += y * y
    if na == 0.0 or nb == 0.0:
        return 0.0
    return dot / math.sqrt(na * nb)


def top_k(query: list[float], vectors: list[list[float]], k: int = 5) -> list[tuple[int, float]]:
    """[(index, score)] sorted by descending cosine; k <= 0 returns []."""
    scored = [(i, cosine(query, v)) for i, v in enumerate(vectors)]
    scored.sort(key=lambda t: t[1], reverse=True)
    return scored[: max(0, k)]
