---
name: uv-run-resyncs-and-strips-installs
description: "uv-managed Python repos: `uv run pytest` re-syncs the venv to the lock and STRIPS anything added by `uv pip install` — install deps, then run via `.venv/bin/python -m pytest`; sys1's service tests additionally need the fastapi + httpx2 extras."
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

feedback 2026-10-27 (sys1, during the clef provider work): `uv run pytest` after `uv pip install pytest` silently removed pytest (uv run syncs the environment to the lockfile), and each diagnostic round-trip cost a re-install. The working shape: `uv pip install <deps>` once, then invoke `.venv/bin/python -m pytest` directly — never through `uv run`.

**Why:** uv's run command treats the venv as derived state and reconciles it against the lock; anything installed outside the lock is drift to be removed.

**How to apply:** in a uv-managed repo, install test deps with `uv pip install`, run tests with `.venv/bin/python -m pytest`. Repo-specific facts hit along the way: sys1's `tests/test_service.py` needs the `[service]` extras (fastapi, uvicorn) AND `httpx2` (starlette's TestClient refuses without it, with its own named error), and the provider-roster pin `CORE` in tests/test_provider_flags.py must gain every new core provider id (the same class as fleet's route-table length pins). Related: [[clef-decision-model-assessment-2026-10-27]], [[fleet-gate-exit-code-trap]].
