---
name: why-the-goal-run-never-delegated
description: "project 2026-11-06 — the owner's live goal run implemented the whole KI ledger without one delegation; the router's execution head judged it `single` over the literal task string (never the plan); the owner's rulings produced the route-judges-the-work plan, now 9/10 IMPLEMENTED and MERGED to origin/master (eb57d8f) — the plan carries its own execution verdict, the work-brief resolver feeds the route, C7's live proof landed (the open-ledger plan judged swarm on the spot, the routed record carrying swarm at 1…"
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

The owner's question (2026-11-06), asked while a live goal run was
implementing the open-ledger closeout: fleet is running a fairly complex
task addressing every issue on the KI ledger, but **no delegation was
used — why?** The run was read in primary source, not inferred:

- Session `20261106T033200Z-14f5a714`, goal "implement plan as designed",
  three legs, ~450 model turns, ~500 tool calls. `subagent` was declared
  in its 25-tool surface and **never called once**.
- **The decisive cause is mechanical:** all three route decisions carry an
  execution head `{"label":"single"}`. `execution_prompt_line` returns
  `None` unless the label is `mixture`/`swarm` (`agent.rs:1979-1994`), so
  **the one line that names `subagent` was never added to the leader's
  prompt.**
- **The judgment's input was the wrong artifact.** The bridge passes
  `&task` straight to the router (`bridge.rs:11406`); `prepare_walk` takes
  `task: &str` and never reads a plan or the goal; the sys1 call is
  `sys1_classify({ "task": "route", "text": task })` (`route.rs:2551`) —
  the task string is the whole of `text`, and the route cache keys on the
  task hash, so the `single` verdict is cached for that literal string.
- **The asymmetry is the tell:** a child's route is judged over its
  **brief** (`prepare_child`, `subagents.rs:1970`) — the actual work. The
  leader's is judged over its **task text**, a pointer to the work.

**The owner's rulings (2026-11-06, in order).** (1) "the single judgement
in this case is obviously wrong … delegation would have been the right
call." (2) "the main session needs to pass to the judge not the literal of
the goal but the implied meaning of it, if it's a plan then the plan needs
to be judged by the tasks and acceptance criteria it holds … GLiDE can
judge intent and maybe we need a composite or stacked judgement."
(3) "normally when I ask to implement a plan it refers to the plan that
the main session recently drafted … if it is a plan that wasn't drafted by
the main session I name the plan either by the full name or part of the
name." (4) "a status of judged, along with draft, active, completed … when
the session picks it up the verdict is already in there and the only
decision the model needs to make is how the delegation will happen … it's
worth adding non-plan work." (5) "if there is a legacy plan that has not
been judged then it must be judged before execution even begins."

**IMPLEMENTED 9/10 and MERGED to origin/master at `eb57d8f`** — see
[[route-judges-the-work-implemented]] for the full mechanism. C7's live
proof landed: the open-ledger plan judged swarm on the spot, the routed
record carrying swarm at 1.0, the nudge firing. C12 (the four decision
rows) is the owner's by the plan's own text.

**How to apply.** The defect was the leader's judgment INPUT, not the
nudge's threshold: any task phrased as a pointer gets a `single` verdict
that is defensible about the sentence and wrong about the work. The
child's brief already did the right thing. Same family as
[[delegation-during-goal-execution-deep-dive]] and
[[the-cap-leaves-the-main-session]].
