---
name: sem1-probe-shadow-second-call
description: "sem1 shipped a probe.py submodule AND a probe function whose own lazy import shadowed it — first probe in a process works, every later one dies with 'module' object is not callable; blend1's semantic lane was dead from its own first re-probe onward. Fixed 2026-10-27 by renaming to probing.py (KI-338)."
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

feedback 2026-10-27 — sem1 shipped `probe.py` (a submodule) and a `probe` function in `__init__`, and the function's own lazy import (`from .probe import run_probe`, inside the function body) imported that submodule — which binds the module over the function in the package namespace. **The first probe in any process worked; every later one died with "'module' object is not callable."** blend1's `/v1/health` carried the failure verbatim for days, so recall and tool_search degraded to the lexical fallback while the lane looked alive at boot.

**Why the diagnosis was confusing:** a fresh interpreter's `sem1.probe("llama-server")` succeeded (768d, ~80ms) while the long-running service's health failed with no crash in its logs. The shadow was state inside the process's module graph, not code and not environment — a fresh interpreter is a first call, the service had made its second long ago. **A green at boot and a failure after the first re-probe is the signature of in-process state, and the test for it is the second call, in the same process.**

**How to apply:** a package cannot reliably hold a submodule and a function of the same name — any submodule import anywhere in the process shadows the function. Fix by renaming the submodule (probe.py → probing.py), and pin it with a test that imports the submodule and asserts the function is still callable (`test_the_probing_submodule_cannot_shadow_the_probe_function`, suite 21 passed). Related: [[sessions-rail-cap-keeps-oldest-200]] (the same session's fleet-side fixes), [[blend1-is-the-latency-triad]].
