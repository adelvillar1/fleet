---
name: composed-joins-the-judged-set
description: "owner ruling 2026-11-02 — \"you can do it if I give you permission to do it, and you have it\": `composed` joins the judged set on BOTH sides (fleet's JUDGED_WORKLOADS and the vendored sys1 lane's ROUTE_WORKLOAD_LABELS), and C18 was proven live — the real lane answered `composed` at 0.892 and the walk landed on blend1; the earlier \"C18 needs the owner's ruling\" was wrong, because the ruling had already been given"
type: project
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

The closeout plan's C18 asked for a live run where the real sys1 lane judges a
task `composed`. I reported it as needing the owner's ruling — `composed` was a
real workload (its own preference entry, its own blend1 candidate) but **not in
`JUDGED_WORKLOADS`**, the closed set the sys1 `route` task answers, so no run
could produce the verdict. The owner's answer: *"you can do it if I give you
permission to do it, and you have it."*

**What landed (2026-11-02, pushed 42b5837):**
- `src/route.rs`: `composed` joins `JUDGED_WORKLOADS` — the fleet side of the
  closed set.
- `services/sys1/src/sys1/tasks.py`: `composed` joins `ROUTE_WORKLOAD_LABELS`
  with its own description — the vendored lane's answer set. **Both sides had
  to move**: fleet's list gates what the judgment may be asked, the lane's list
  gates what it may answer.
- `tests/composed_live.rs`: drives the real lane over the real route
  (`select_walk_with_home`) and asserts the verdict is `composed` and the walk's
  first candidate is `blend1`. The lane's own answer for the probe task was
  `composed` at **0.892** — before the amendment it answered `standard_code`,
  which is exactly the failure C18 described.

**Why:** the "not mine to close" framing was wrong in both directions. A
criterion whose blocker is a *closed set* is a one-line ruling, and the owner
had already given it — the earlier report treated "the label is not in the set"
as a reason to stop rather than as the thing to change. The permission was
already in hand; the work was mechanical once the ruling was read as granted.

**How to apply:**
- When a criterion is blocked by a closed set, a label list, a vocabulary or a
  gate, check whether the owner has already granted the change — "you can do it
  if I give you permission" means the permission is the default, not the
  exception.
- A judged set lives in **two** places when the judgment is a vendored lane:
  the caller's allowlist and the lane's task labels. Changing one without the
  other leaves the run failing the same way.
- Prove the amended set live when the lane is reachable: the lane's own answer
  is the evidence, and a mock that answers what the test expects proves nothing
  about the label set.

Related: [[open-work-closeout-2026-11-02]],
[[a-ruling-has-to-name-what-the-check-can-see]],
[[right-fixes-not-easy-fixes-beyond-the-plan]],
[[a-plan-never-ticked-is-not-a-status]].
