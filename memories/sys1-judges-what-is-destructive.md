---
name: sys1-judges-what-is-destructive
description: "owner ruling 2026-11-03 — sys1 should judge what is destructive, not a static class list; the destructive directive itself stays, the verdict rides the seat as data, and the owner SCOPED it to the actions the standing class already flags (one lane call per held action, not every write-capable call)"
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

Owner ruling 2026-11-03, on the huddle's approval story, verbatim: *"the
destructive directive must remain, I also think that sys1 should judge what
is destructive or not."*

**What stays:** the binary rule the owner set on 2026-10-28 — non-destructive
work runs without asking; destructive work is held every time and flagged
with a visual cue and no always-allow
([[one-goal-many-personas-destructive-is-the-only-gate]]). The huddle
inherits it rather than inventing an approval story.

**What changes:** the *classifier*. Today destructiveness is a **static
class list** — `Gate::is_destructive` names `write_file` and the bash
destructive list, and the bot face's `guarded` mode is that list applied
per call. The owner wants the judgment made by **sys1 per action** instead:
an action's reversibility answered by the judgment lane, not by a name on a
list.

**The seam already exists and is the reason this is buildable.** The seat
document carries `destructive` as **data** on both delivery paths
(`describe_seat` for the event, the state document's `approval`), so the
face's red cue is the gate's decision, never the client's guess. A sys1
verdict lands in exactly that field — the face needs no new language, only
the gate's answer. The hard part is the judgment itself: `bash` destructiveness
is already per tool and argument shape with a conservative default, because
a command's reversibility cannot be answered statically — which is the same
argument the owner is now making for the whole class.

**The plan's reading (2026-11-03): it rides globally, not only in
huddles — and the owner approved it that way.** `docs/plans/2026-11-03-huddles.md`
C10 changes the approval gate's classifier everywhere, because the gate is
one mechanism; the plan flags the global scope in its Risks and the owner
approved the plan with `/goal implement plan as designed`. The lane-down
fallback must **fail closed** (an unjudged action is destructive until sys1
says otherwise) and name the degradation on the seat. The seat mockup
(`docs/plans/mockups/2026-11-03-huddles/seat.html`) renders the verdict as
data — stamp, reason, lane, latency, confidence — beside a contrast row
showing a read that passed without a seat, because the judgment is per
action, not per member or per huddle.

**The owner settled the scope (2026-11-03, when asked): sys1 judges ONLY the
actions the standing destructive class already flags** — *not* every
write-capable call. One lane call per held action, so nothing changes about
which calls ask; the judgment explains the ask with a reason on the seat. The
wider reading (every write-capable call) can catch what the static list
misses but spends a lane call on nearly every action and changes what
"held" means app-wide — the owner chose the cheap, explaining form, and the
calibration stream can later show whether the class is missing anything.

**C10 is now BUILT** ([[huddle-plan-implemented-12-of-14]]): at the seat,
sys1 judges and the verdict rides the seat event and the state document as
data — `destructive`, `reason`, `confidence`, `judged_from`. A lane that
cannot answer stands the class in and **says so on the seat**, so a missing
judgment is never mistaken for one. The seat mockup's contrast row (a read
that passed without a seat) is the point: the judgment is per action, not per
member or per huddle.

**Why:** a static list is a guess made once and frozen; the owner's
autonomy contract is per-action reversibility, and a judgment lane is how
fleet already answers questions it cannot answer statically (routing,
delegation grading, the curator's vote). **How to apply:** treat this as a
reopening of the 2026-10-28 ruling's classifier, not of its rule — the
binary stays, the list becomes a judgment, and the seat's `destructive` field
is the contract that lets the face stay dumb. Name the conservative default
explicitly (an unjudged action is destructive until sys1 says otherwise),
because a judgment that fails open is a silent data-loss path. Related:
[[one-goal-many-personas-destructive-is-the-only-gate]],
[[huddle-surface-ideation-2026-11-03]] (the surface this ruling was made
for), [[a-dated-ruling-supersedes-a-pinned-tests-reason]] (the pattern of a
dated ruling superseding a pinned test's reason).
