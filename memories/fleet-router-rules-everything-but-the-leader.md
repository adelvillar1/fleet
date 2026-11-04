---
name: fleet-router-rules-everything-but-the-leader
description: "owner rulings 2026-10-21 (built 4bd2f18): the leading session walks hard, every other route is the router's by judged difficulty, deepseek-flash closes every walk, no model is a hardcoded default, --model is a pin; the one boundary and the one named gap"
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

Four owner rulings on 2026-10-21, after the router deep dive, built and
pushed in `4bd2f18` against `docs/plans/2026-10-21-router-rules-everything-else.md`
(approved before the build — the owner corrected an earlier attempt to build
without presenting the plan):

1. "The default for everything should be the router unless I override it" —
   no provider's model is hardcoded; `api::DEEPSEEK_MODEL` names only the
   reserve entry, surfaces seed `route::AUTO_MODEL`, and `--model` is a pin
   resolved by name (`route::resolve_model_pin`; a bare id on two providers
   is refused listing both spellings).
2. "The router invokes the lanes and the models based on difficulty" —
   narrowed when asked how it squares with the 2026-10-16 orchestrator
   ruling: **"the orchestrator, the session that leads should be set to
   hard, everything else the router rules."** `Walk::Leader` keeps hard's
   chain; a delegated child goes through `route::prepare_child`
   (`Walk::Judged`) over its own brief under the parent's pin, on its own
   routed client. This supersedes hard rule 9's "children inherit the
   parent's model".
3. A provider pin narrows the router to that provider's models (already so).
4. "If all models from the router fail then deepseek-flash is the failover"
   — `route::with_tail` appends the reserve to every order; the row says
   `served by the failover tail` when it serves. This replaced the
   "reserve serves only when pinned" policy.

Also: `deepseek-chat` → `deepseek-flash` everywhere (the provider kept the
old name as an alias; the owner renamed anyway; ledger and dated records
keep the old name as history).

**Why:** these are standing rules the code now embodies; a future session
must not re-derive the old orchestrator-everywhere or pin-only-reserve
behaviour from older docs or memories.

**How to apply:** the one boundary — a child routes itself only when its
parent's client is routed and the run has a home; an unrouted parent
(every test double) hands the child the same client. The one named gap is
**KI-278**: a scheduled run's child keeps the parent's client because cron's
`improve_home` is `None` by design and doubles as the route home — fix is a
distinct route home. Deferred by name: `composed` is unreachable by judgment
(sys1's five labels), and effort-per-workload is its own plan (research in
[[fleet-owner-dogfooding-phase]]'s session). See also
[[orchestrator-walks-the-hard-chain]], [[router-enforcement-ruling]].
