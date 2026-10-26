---
name: blend1-refresh-needs-explicit-absolute-sources
description: "blend1 refresh over MCP needs explicit ABSOLUTE sources globs — the default docs set resolves against the lane-home cwd and derives nothing (action "no-source")"
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [drops-the-row]
phrases: [refresh, no-source, BLEND1_SEM1_PYTHONPATH, sources absolute glob, docs/**/*.md, KI-223 absolute keys]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

blend1's MCP refresh resolves `sources` against the child's cwd — the lane home services/blend1/ — where the default docs set (docs/**/*.md + root contract docs) matches nothing, so an omitted sources returns action "no-source" with 0 items instead of refreshing the project. A refresh call must pass sources explicitly as ABSOLUTE globs, e.g. ["/Users/alejandrodelvillar/Projects/<repo>/docs/**/*.md"] (+ any root contract docs that exist). Absolute paths are also the cwd-independent key form since KI-223, so an explicit absolute glob is what keeps a rebuild from churning the whole store. Verify the sibling wiring first (BLEND1_SEM1_PYTHONPATH → services/sem1/src is mandatory for refresh; its absence is a named -32603 error, KI-310), and read the target project's existing store manifest (~/.local/share/blend1/vectors/<project>/embeddinggemma-2/manifest.json) to confirm the source set before calling.
