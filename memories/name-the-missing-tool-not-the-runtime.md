---
name: name-the-missing-tool-not-the-runtime
description: "owner correction 2026-10-18 — "What do you mean by no python on this machine. Of course there is python" — I wrote "no pytest on the machine" and it read as "no python"; python3 was present and in use all session"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

When a tool is absent, name the SPECIFIC missing tool — never let the
phrasing imply the interpreter or runtime is absent. During the 2026-10-18
KI triage I wrote "the lane's own pytest is not installed on this machine"
while running python3 in the very same commands; the owner read it as a
claim that the machine has no Python and corrected: "What do you mean by no
python on this machine. Of course there is python."

**Why:** an overstated absence is a false statement about the user's own
machine, and it undersells the verification I actually did — the blend1
lane's seven tests ran and passed directly under python3 once I supplied
the `tmp_path` fixture pytest would have. The honest sentence was "pytest
the runner is absent, so the suite runs directly under python3", which is
both true and a stronger claim.

**How to apply:** before stating that anything is missing, check what IS
present (`command -v`, `python3 -c "import …"`), and name the gap at the
granularity of the actual gap — a runner, a library, a binary — not the
language. If a test suite can run without its framework, run it that way
rather than reporting the framework's absence as the end of the road.
Related: [[fleet-ki-triage-pass-one]] (where this happened),
[[owner-stated-facts-are-authoritative]].
