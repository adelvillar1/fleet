---
name: session-recap-2026-11-01-design-parity-and-knowledge
description: "project 2026-11-01 — the recap written and pushed (6012b76) via a temp branch onto master's tip (local master held by the parallel session's worktree), the face reloaded from the merged build and byte-proven, cache sweep 29.8→21.3 GiB debug; the relayed faces' secret lives only in the owner's shell"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-11-01 wrap-up of the design-parity + project-knowledge build (both
plans complete, merged `ecbf280`): the session recap at
`docs/recaps/2026-11-01-design-parity-and-knowledge.md`, pushed as
`6012b76`.

**The recap landed on a temp branch, not a checkout.** Local `master` is
held by the parallel session's `fleet-master-wt` worktree (with
uncommitted settings-ux edits) — `git checkout master` refuses from any
other worktree, and committing into THEIR tree risks sweeping their WIP.
The move that worked: `git checkout -b recap-tmp origin/master`, commit
the one file with explicit paths, `git push origin recap-tmp:master`,
delete the temp branch. The remote advances; their local master catches
up on their next pull (the recap file is not one of their edited paths,
so it merges clean). Same class as [[a-parallel-session-co-edits-your-file]]
but for refs instead of hunks: **never check out a branch another
worktree holds — branch from its tip and push the ref.**

**The reload:** `pkill fleet gui` + nohup relaunch, then prove the served
bytes (index 200, `design-float` present in the served app.js, the site
route 200 with rewritten page links). ~~The relayed faces' secret lives
only in the owner's shell~~ — SUPERSEDED same evening: the owner said
"relaunch it… if we need to set a new secret that's fine", so both relay
secrets were ROTATED (OpenShip `env set`+`env apply` on `relay`/`bot`,
both `FLEET_*` proofs written to `~/.fleet/.env` AND the plist env), the
launchd face rejoined both relays, and the root cause behind the whole
ordeal was fixed as KI-393 at `737dadc` (the join-proof check ignored
`.env` — see [[relay-join-secret-rotation-2026-11-01]]).

**Recap-step numbers:** ledger green (392 rows, 26 open — the parallel
session's new filings rode in), cache sweep debug 29.8 → 21.3 GiB (dev
profile cleaned whole, over the backstop), free space 31.1 → 39.6 GiB,
release untouched. Related: [[design-parity-plan-2026-11-01]],
[[designs-are-project-knowledge-plan-2026-11-01]],
[[fleet-recap-ends-with-cache-sweep]].
