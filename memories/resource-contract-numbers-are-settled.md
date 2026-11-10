---
name: resource-contract-numbers-are-settled
description: "The resource contract's declared numbers stand as written — the owner's 2026-11-10 ruling \"leave them like that\" — so no session asks again: 2 GiB memory ceiling, 80 processes, 20 GiB free floor, reserve share tight 12% / balanced 8% / roomy 4%, and the per-class estimates (release-gate and release-assets 8 GiB, lane-bootstrap 2 GiB, a run's own budget 2 GiB)."
type: feedback
date: 2026-11-10
basis: confirmed-by
tier: durable
shape: [docs, resolved]
phrases: [leave them like that, proposals until the owner confirms them, reserve share tight 12% balanced 8% roomy 4%, ESTIMATE_RELEASE_GATE_KIB, MEMORY_CEILING_KIB PROCESS_CEILING DISK_FLOOR_GIB]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The resource contract (docs/plans/2026-11-09-the-resource-contract.md) shipped with its numbers marked "proposals until the owner confirms them", and the handoff listed confirming them as an open decision only the owner could make. Asked on 2026-11-10, the owner's answer was **"leave them like that"** — the declared values are the contract's numbers, not placeholders.

Settled as written:

| Number | Value | Declared in |
|---|---|---|
| Memory ceiling | 2 GiB | `src/footprint.rs` MEMORY_CEILING_KIB |
| Process ceiling | 80 | `src/footprint.rs` PROCESS_CEILING |
| Free floor | 20 GiB | `src/doctor.rs` DISK_FLOOR_GIB |
| Reserve share: tight / balanced / roomy | 12% / 8% / 4% of the volume | `Appetite::reserve_share` |
| Estimate: release-gate, release-assets | 8 GiB each | ESTIMATE_*_KIB |
| Estimate: lane-bootstrap | 2 GiB | ESTIMATE_LANE_BOOTSTRAP_KIB |
| A run's own budget | 2 GiB | RUN_BUDGET_KIB |
| Store budgets | target 20, one lane 20, lanes 24, ~/.fleet 12 GiB | scripts/footprint.sh |

Two rules still constrain them and are not up for inference: the reserve never learns downward (only estimates and per-work budgets calibrate), and a habitually tight machine tightens fleet rather than loosening it.

The documents were updated in the same session so the next reader is not asked the same question: the plan's Risks bullet, the handoff's "Reserve fractions… are proposals" paragraph and its "open decisions" list, the ESTIMATE_* doc comment, and docs/features/resource-contract.md. A change to any number is the owner's edit in the one place it is declared.

The other half of that turn's question — on a deterministic reserve breach, hard-stop or pause for a yes — was NOT answered; the contract hard-stops today, which is the plan's default, and that remains the owner's open call.
