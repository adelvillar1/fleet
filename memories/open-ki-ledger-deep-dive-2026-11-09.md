---
name: open-ki-ledger-deep-dive-2026-11-09
description: "Deep dive on the ledger's 11 open KI rows, landed as docs/architecture/deep-dive-2026-11-09-open-ki-ledger.md (527767b) — four clusters, each row re-verified live, with the recommended work order"
type: project
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

Deep dive 2026-11-09 on the open KI ledger (501 rows, **11 open**: 7
defects, 2 gaps, 2 improvements), landed as
`docs/architecture/deep-dive-2026-11-09-open-ki-ledger.md` at `527767b`
(docs-only). Four clusters, each row re-verified against the live tree:

- **A — one upstream pair, three rows**: KI-396/455/456 are
  dev-decisions' D-001 (silent 2000-char section cap) + D-002
  (criteria parser counts every checkbox, shifting ids), both still
  open there; no vendored gate copy in fleet's tree, so the fix is
  purely upstream and closes three rows.
- **B — the live finds**: KI-493 still live NOW (port 49473 refused,
  docker store still empty — every delegation dies until the owner's
  `docker pull` + `fleet install`, see [[layerfs-worker-dies-with-dockers-data]]);
  KI-492's silent none-arm stands at `bridge.rs:4670` (see
  [[runs-refuse-the-filesystem-with-no-folder]]); KI-494 AGED — the
  `computer-use` mcp.json entry now HAS a `program`, but it is a
  RELATIVE path to a SkyComputerUseClient bundle that exists nowhere on
  the machine; the CuaDriver doctor resolves is present.
- **C — deck-vector pair**: KI-252's clustering genuinely unbuilt
  (`grep -ci cluster` = 0 in deck.rs/memory.rs); KI-459's harness times
  the embed not the ranking, so `memory.vectors.two_stage` sits
  default-OFF on an unreadable measurement; fix = a vector-taking seam.
- **D — owner rulings**: KI-476 Send-button contrast re-measured
  byte-identical (console 3.06:1, graphite 2.93:1; only the two dark
  themes' gradient stops need darkening; natural ride-along for the
  next face cut — 0.6.18 is committed but NOT built/installed, face on
  3080 still answers 0.6.17); KI-480 title ladder still 4 rungs
  (owner/goal/plan/first); KI-466 fact-name grammar unchanged.

**Recommended order**: KI-493's pull → cluster A upstream → KI-459's
seam → KI-492 + KI-476 on the same face cut → the rulings → KI-252 as
its own plan.

Also delivered this session: 0.6.18 state check — repo at 0.6.18,
release binary/dmg/face all 0.6.17, one commit ahead of origin
(`15fa424`), and the latest recap covers only through 0.6.17.
