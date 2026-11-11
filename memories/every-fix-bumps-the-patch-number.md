---
name: every-fix-bumps-the-patch-number
description: "The owner's standing release rule: any fix that produces a new build bumps the patch number, and the 0.6.x -> 0.7.y decision is theirs to announce in advance"
type: feedback
date: 2026-11-11
basis: confirmed-by
tier: durable
shape: [deploy, resolved]
phrases: [if we fix something and a new build is created we bump the number, 0.6.x to 0.7.y I would let you know in advance, you have my explicit approval from now on, the three declarations move together, doctor binary line compares bytes not the version string]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

Owner ruling 2026-11-11, given as an explicit standing approval: "if we fix something and a new build is created we bump the number" — so a fix that lands a new binary always carries a patch bump (0.6.26 -> 0.6.27), never a silent rebuild under the same version. A larger-scale change that merits moving to 0.7.y is announced by the owner in advance, not decided by the session. "Otherwise just do it, you have my explicit approval from now on": the session bumps, cuts, pushes, builds assets, installs and reloads without asking each time.

The bump is one release commit moving the three declarations together (Cargo.toml, src-tauri/Cargo.toml, src-tauri/tauri.conf.json) plus Cargo.lock and a CHANGELOG.md section, exactly as CHANGELOG.md's own header states. The reason this rule exists: 0.6.25 and 0.6.26 were both deployed while the version stayed put, so `GET /api/version` and the doctor could not tell a stale install from a fresh build — the same defect `reload::reconcile_state` names (KI-412), now surfaced as doctor's `binary` line, which compares the files' bytes rather than a version string.
