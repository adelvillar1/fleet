---
name: lane-child-path-is-inherited-not-declared
description: "A lane launch spec declares no PATH — mcp::BASE_ENV copies the parent's PATH, HOME, LANG and TMPDIR — so a lane child's python3 is fleet's own launch environment, not a spec constant."
type: project
date: 2026-10-28
basis: inferred
tier: durable
shape: [build, deps]
phrases: [BASE_ENV copies the parent PATH, src/mcp.rs:119 BASE_ENV, launch spec declares no PATH, env -i PATH=/usr/bin:/bin:/usr/local/bin python3 -V 3.9.6, /usr/local/bin/python3 3.13.6, zip(strict=) Python 3.10]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

src/mcp.rs:119 `const BASE_ENV: &[&str] = &["PATH", "HOME", "LANG", "TMPDIR"]` and child_env (:132-136) copies each *when the parent has it*; src/lanes.rs `launch_spec` adds only the lane's `<SVC>_PYTHONPATH` (plus blend1's four sibling path vars) and its passthrough names. Nothing pins PATH.

Consequence, measured 2026-10-28: `env -i PATH=/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin python3 -V` → 3.9.6 (/usr/bin/python3), while a shell with /usr/local/bin first → 3.13.6. So whether KI-324's `zip(..., strict=True)` refusal (Python 3.10+) fires on a rebuild depends on how fleet itself was started, not on the spec — a launchd-started fleet can hand its child 3.9.6 while a hand-driven catch-up in the owner's shell gets 3.13.6. This refines KI-324's premise, which reads as though the spec sets "the launch spec's PATH".

The durable fix therefore cannot be a PATH edit in the spec: it is a pinned interpreter (venv) or a startup refusal in mcp_server.py when sys.version_info < (3, 10). A hand-driven driver should pass sys.executable explicitly and read back `python3 -VV` rather than assume.
