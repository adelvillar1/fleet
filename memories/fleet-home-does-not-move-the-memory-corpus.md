---
name: fleet-home-does-not-move-the-memory-corpus
description: "FLEET_HOME does not redirect the memory corpus — resolve_root_from checks the home corpus only if it holds facts, then walks the working directory; a scratch-home probe wrote into the repo corpus"
type: reference
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

`resolve_root_from` (src/memory.rs:734-781) resolves the corpus in
this order: the fleet home's own `memories/` **only if it holds
facts** (an empty scaffold is skipped — the shadow rule), then the
ancestor walk from the working directory, then `cwd/memories`.

**Why:** a scratch-home probe (2026-11-05, the desk live proof) ran
`fleet remember` with `FLEET_HOME=/tmp/scratch` from the repo's
working directory and the fact landed in the REPO's `memories/` as
debris — the scratch home held no corpus, so the walk found the repo's.

**How to apply:** scratch-home memory work needs the working directory
inside the scratch home, not just `FLEET_HOME` pointed there; and
never seed `~/.fleet/memories` with anything — the moment the home
corpus holds facts it shadows every project corpus on the machine
(the house-methodology plan's named hazard). The transplant and
house-seed scripts write through the project's working directory.
See also [[memory-transplant-zcode-to-fleet-proposed-2026-11-04]].
