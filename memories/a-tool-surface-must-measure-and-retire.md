---
name: a-tool-surface-must-measure-and-retire
description: "feedback 2026-10-23 — after drafting a plan that only ADDED ten tools, the owner's amendment was \"we need to measure and retire the tools that don't make sense to keep\"; a surface needs a lifecycle (declaration record, usage economics, a trial window, DISABLE-not-delete) or it only grows"
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

The plan I wrote for fleet's tool gaps added ten tools, moved the
declaration bound from 28 to 32, and listed what was out of scope. It had
no answer for the opposite question. Owner: **"we need to measure and
retire the tools that don't make sense to keep, update the plan with
that."**

The surface had already been through this once, badly: the advisor's first
"declared and never called" rule fired on every tool added that day,
because the log records a catalog *count* and a call count, never the
declaration list. With no denominator (number of runs that declared the
tool) and no age, "unused" and "new" are the same row. I removed that rule
rather than fix the data, which was the right call for the rule and the
wrong call for the surface.

**Why:** a tool surface only grows unless something measures it, and a
measurement that cannot tell new from unused will retire the work it was
meant to curate. My own first rule did exactly that.

**How to apply:** when a plan adds capabilities, budget the lifecycle in
the same breath — what each addition costs to declare, how its use will
be counted, and what would remove it. Concretely: record what each run
declared (the denominator), measure cost and outcome per tool, give new
tools a trial window so age is part of the verdict, and **disable rather
than delete** — the owner's own words, "we just disable tools that are not
used. this allows us to activate it if there is a future need." A disabled
tool is not declared and is out of `tool_search`, but a call to it answers
"disabled by the owner", so aliases, resumed sessions and saved plans
still resolve; one config write brings it back with no rebuild. Never let
a threshold delete code: a rare tool can be decisive, a foundational one
can look quiet because a sibling covers it, and a fallback is only reached
when the primary fails — which is why a protected set exists rather than a
pure usage threshold. The same measurement that disabled a tool should
watch for its return and suggest the re-enable.

**The escalation, and its general lesson:** the owner then widened it —
**"because the disabling is not a destructive action I'm okay with the
harness disabling the tools that are least used. the same could probably
apply to skills, workflows, and loops."** The mechanism is what earned the
autonomy: an action that is reversible, guarded (protected set, trial,
a cap per sweep, a dry run, a calibration row, a cooldown after an enable)
and observable is one the owner will let run unattended, where a proposal
they must click is only worth asking for when the action cannot be undone.
So design the reversible form first, then ask whether the harness may act —
and when it may, apply it to every similar surface rather than just the one
that prompted it. Keep auto-*enable* out: suggestions there, the owner
decides.

**Then the owner caught a hole in my own sweep of "every similar surface":**
I had written workflows off as having no registry —
**"I thought the router had some workflow functionality already built in."**
It does: the router's named workload chains (quick, standard_code, hard,
prose, deep_context, vision, composed) are selected every run and recorded
per run, i.e. a declared/used/disable/enable unit that already existed.
The lesson generalises past this plan: when the rule is "cover every similar
surface", each surface it claims does not exist is a claim to check against
the obvious subsystem first — an unexamined "nothing to measure here" is how
a lifecycle silently misses the one surface with the most history.

**The mirror, and where the line is:** the owner then asked for the other
direction — **"conversely if we have a work pattern that has been repetitive
we should suggest a new workflow, same as skills, tools, and loops"** — and
the archive already holds the evidence: one session vector per conversation,
plus each session's task and call sequence. So repetition is a *creation*
signal with the same shape as no-use is a removal signal, and the same
observation (a cluster in the archive) reads in both directions. The two
are not symmetric, though, and the asymmetry is the rule: **disable** is a
reversible config flip with guarded evidence, so the harness may do it;
**create** changes the prompt and the surface, so the harness only
proposes. Design any lifecycle pair that way — the reversible half can run
unattended, the additive half stops at a suggestion with its evidence.

Related: [[fleet-tool-availability-gaps-2026-10-23]],
[[a-fixture-corpus-flatters-a-retriever]] (a threshold is a measurement),
[[fleet-skills-invocation-is-model-only]] (0 of 442 skills used — the
earlier, owner-side retirement direction for a different surface).
