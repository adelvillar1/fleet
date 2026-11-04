---
name: a-criteria-cap-makes-qa-green-by-omission
description: "feedback 2026-11-01 — the plan gates silently capped a plan's own shape (criteria at 12 in plan-gate/evidence-gate/plan-surface and sys1's parse_criteria, sections at 14, arch-gate claims at 12), so criteria past the cap were never judged; the owner ruled "a plan needs however many criteria it needs to pass" — the caps are removed, and a cap you cannot see is a cap you have not measured"
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

2026-11-01, found while gating `docs/plans/2026-11-01-design-parity.md`
(a 14-checkbox plan): **every gate read only its first 12 criteria.**
plan-gate printed the honest-looking line *"no heads for 2 more
criteria"* and plan-surface reported "12 criteria" — C13 (the local
host, the plan's biggest new Rust surface) and C14 (close-out) were
invisible to coverage, scope, deps, mapping and (by extension) the
evidence gate's `== C<i> ==` numbering. The cap lived in three layers,
each hiding the next: `_PLAN_GATE_MAX_CRITERIA = 12`,
`_PLAN_GATE_MAX_SECTIONS = 14`, `_EVIDENCE_GATE_MAX_CRITERIA = 12`,
`_ARCH_GATE_MAX_CLAIMS = 12` in `dev_decisions.py` **and** its
refactored twin `devdec/gates.py` (the shim runs the former; both had
to be fixed), plus `MAX_CRITERIA` in **sys1's**
`plansurface.parse_criteria`, which returned `out[:12]` — so even a
caller that stopped slicing still got 12.

**Why:** the owner's ruling, verbatim in spirit — *"the 12-cap is
ridiculous, we need to get rid of that, a plan needs to have however
many criteria it needs to pass."* A ceiling on the plan's own shape is
not a safety bound; it converts QA into green-by-omission: the gate
reports "GAPS" over a subset and the plan's tail never faces a judge.
The overflow report is the tell — a tool that *knows* it dropped work
and says so in a footnote has already decided the plan's contract for
it. (The three-layer spread also means fixing the caller alone does
nothing: sys1 truncated independently.)

**How to apply:** when a gate's count disagrees with the file's
checkbox count, the gate is lying by truncation — count the criteria
yourself (`grep -c '^- \[ \]'`) and compare against the header line.
Remove caps on the plan's own expressiveness (criteria, sections,
claims) outright; keep caps that bound *inputs the plan does not own*
(test-file scans, evidence-block chars, docs-gate section reads) —
those bound prompt size, not the contract. `PAIRS_PER_REQUEST` in
plansurface stays for the same reason (a transport batch, not a
ceiling). After the fix: plan-gate and plan-surface read 14/14 on the
same plan, arch-gate 15 claims; after the fan-out amendment the plan
grew to 16 criteria and plan-gate judged all 16 (C15/C16 covered) —
uncapped means the count follows the contract. Filed as **KI-392** via
`fleet ki append` (closed with evidence) — the verb is `--next`, not
`--next-action`, and its writer appends one row without re-rendering
the table, so it never normalizes existing rows. The skill library and
the dev-decisions repo are **hardlinked** (one inode — `cp` refuses
"identical"), so one edit covers both homes. **Committed and pushed
2026-11-01:** fleet's plan `9d89c98`, dev-decisions gates `57d3ad0`
(its `main`), sys1 `1508b09` (its `feature/plan-surface` — that repo
carried other sessions' uncommitted `config.py`/`providers.py`/tests,
so only `plansurface.py` was staged, explicit paths); pushed
`360773c..314a7f4` / `039cea1..57d3ad0` / `f0faebc..1508b09`. A
parallel session's severity normalization swept the ledger including
the KI-392 row into `16600fe`. Fleet's vendored `services/sys1` copy
still carries `MAX_CRITERIA` until the next vendor sync. Related:
[[plan-gate-indexes-criteria-zero-based]],
[[design-parity-plan-2026-11-01]].
