"""KI-347's pin: the query path carries its OWN budget.

`search` used to call `sem1.embed([query])` bare, riding http_llama's
inherited 60-second default — so whenever the one-slot embedder was
busy behind a corpus rebuild, a five-token query timed out and the
failure surfaced as "endpoint unreachable": a queue wait reported as
an absence. The pin captures the kwarg the search actually passes —
the budget is the contract, so it is asserted as a value, not
inferred from behavior.

Run: python3 services/blend1/tests/test_search_budget.py
"""

import os
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
sys.path.insert(
    0, str(Path(__file__).resolve().parent.parent.parent / "sem1" / "src")
)

from blend1 import corpus  # noqa: E402
import sem1  # noqa: E402
from sem1.types import EmbedResult  # noqa: E402


def test_the_search_passes_its_own_budget():
    """The search's embed call carries EMBED_QUERY_TIMEOUT_SECONDS —
    captured at the seam, so the value cannot drift with a rewrite of
    either side."""
    seen = {}
    real_embed = sem1.embed

    def spy(items, **kwargs):
        seen.update(kwargs)
        seen["items"] = list(items)
        return EmbedResult(
            vectors=[[0.0, 0.0, 1.0]],
            dims=3,
            model="stub",
            provider="stub",
            telemetry={},
            extras={},
        )

    sem1.embed = spy
    store = tempfile.mkdtemp(prefix="fleet-search-budget-")
    previous = os.environ.get("BLEND1_VECTORS_DIR")
    os.environ["BLEND1_VECTORS_DIR"] = store
    try:
        # An unseeded store answers [] cleanly — no retrieval, no disk
        # dependency on this machine's real corpus.
        hits = corpus.search("a single number that carries the point")
        assert hits == [], hits
    finally:
        if previous is None:
            os.environ.pop("BLEND1_VECTORS_DIR", None)
        else:
            os.environ["BLEND1_VECTORS_DIR"] = previous
        sem1.embed = real_embed

    assert seen.get("timeout") == corpus.EMBED_QUERY_TIMEOUT_SECONDS, seen
    assert corpus.EMBED_QUERY_TIMEOUT_SECONDS == 120.0
    assert seen.get("items") == ["a single number that carries the point"], seen


if __name__ == "__main__":
    test_the_search_passes_its_own_budget()
    print("ok: the search carries its own 120s budget")
