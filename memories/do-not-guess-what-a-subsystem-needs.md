---
name: do-not-guess-what-a-subsystem-needs
description: "feedback 2026-11-03 — the owner rebuked confident guesses stated as facts about fleet's own subsystems (\"for fucks sake, fleet uses sys1 extensively! why are you asking for a key there\"); read the subsystem before claiming what it needs, and never present an assumption as a blocker"
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

The owner's rebuke, verbatim: *"before talking nonsense about sys1, why don't
you read what sys1 does in the context of fleet? I don't appreciate confident
guesses stated as facts, do a deep dive on how sys1 works on fleet and you
will see it's never been blocked."*

**What I got wrong, twice, in the same turn.** I said C10 "never needed a key
at all — sys1 is a vendored lane" (half right: the lane is local, its **judges**
are ten hosted keys), and I said C14 "needs a working provider key" (wrong: the
providers are already configured on this machine, and a scratch home can carry
them in its **environment**). The second was worse because I presented it as a
blocker requiring the owner's input — an assumption dressed as a constraint.

**The deep dive, once done, is short and decisive:**

- **sys1 is a vendored Python lane** — `services/sys1`, spawned locally over
  fleet's own MCP client, exposing `sys1_classify` and
  `sys1_perceive_document`, answering in ~60 ms and probing `✓ ok` on this
  machine right now (`fleet lanes`).
- **Its judges are hosted**, not local: `SYS1_DECISION_KEYS` is ten keys
  (FASTINO, TYPESAFE, DREX, SUPERSONIC, LIQUID, CLM, OPENJEV, KEV, TOGETHER,
  LAYA), read from the process env or `<home>/.env`.
- **Fleet uses it constantly**: every run's workload judgment
  (`route::judge`), `fleet judge`, the calibration gate and `anomalies`, and
  `tool_router`'s capability head.
- **It has never been blocked** because its absence is a *named fail-open*, not
  a wait: `sys1_judges()` checks whether any of those keys resolves; if none
  does, the spawn is **skipped rather than paid for** (KI-044) and the router
  falls back to the declared HEAD **by name**. A keyless home is the designed
  degradation path, not a blocked state.

**Why:** the owner's system is the thing they know best, and a confident wrong
claim about it costs more than a question — it manufactures a blocker, and the
next session inherits it as a fact. **How to apply:** before saying what a
fleet subsystem needs, read it — its lane spec, its key list, its fail-open
path. If the answer is not in the code, ask; never state an assumption as a
constraint, and never let "needs a key" stand unverified when the question is
one `fleet lanes` or a grep away. Related:
[[sys1-pool-is-all-hosted-no-extraction-wire]] (the earlier instance of the
same mistake — "run `sys1 doctor` before asserting what the pool can serve"),
[[verify-the-binary-is-newer-than-the-source]] (check the artifact before
theorizing), [[a-ruling-has-to-name-what-the-check-can-see]] (verify the
artifact a claim names exists), [[huddle-plan-implemented-12-of-14]] (the
session where this happened).
