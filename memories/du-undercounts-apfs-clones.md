---
name: du-undercounts-apfs-clones
description: "feedback 2026-10-27 — du showed fleet-remote's target at 7.7 GB but cargo clean reported and removed 25.9 GiB; APFS clonefile/du block accounting makes du a wrong instrument for clone-heavy build trees, so record the REMOVAL TOOL's own accounting as the reclaimed number, and never compare a before-du to a tool-reported after"
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

**2026-10-27, the projects repo sweep:** `du -sk` measured the
fleet-remote worktree's `target/` at 7.7 GB and the plan promised
"~21.7 GB" of reclamation. `cargo clean` then reported and removed
**25.9 GiB** from that one worktree (10,335,9 files), and the
machine-wide du moved 60.4 → 38.2 GB — more than du's own before/after
arithmetic could explain.

**Why:** APFS copy-on-write clones make a file's logical size larger
than the blocks du counts, and build tools (cargo, npm) clone heavily;
`du -sk` reports st_blocks, so clone-heavy trees read smaller than the
bytes the tool actually manages and frees.

**How to apply:** when a cleanup's reclaimed number matters, take it
from the removal tool's own report (`cargo clean`'s total, `npm`/
package-manager output) or from a before/after of the CONTAINER's
bound (`df`, quota), never from `du` arithmetic over clone-heavy
trees — and expect a plan's size promise built on du to understate the
win rather than overstate it. The same asymmetry means a "surprisingly
cheap" directory on du may be the biggest lever, not the smallest.
