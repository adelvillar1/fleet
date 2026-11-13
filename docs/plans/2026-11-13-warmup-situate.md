---
title: the warmup is situate-and-align — turn economy, the execution ruling, the corpus root
date: 2026-11-13
status: completed (2026-11-13; the ruling landed in prompt, skills, orientation docs and corpus; the corpus root follows the session folder; KI-533..536 filed)
---

# The warmup is situate-and-align — turn economy, the execution ruling, the corpus root

The owner ran fleet's own warmup (session `20261113T153658Z-5da6f173`) and
it failed the product's own bar: 60 turns, ~10.7 minutes, 96 model calls,
158 tool calls (147 bash), prompt tokens 10k → 117k — and it ended by
self-authorizing work ("I'm picking up the housekeeping in dependency
order… already authorized"). The owner corrected it on the record: warmup
is "just for getting situated and align your bearings with what's going on
with the project", and "plans govern work only when I activate them or I
request to continue working, a plan even if it's active is not a license
to jump into work, you need to ask me if I want to do that". The agent
confessed: "That was me granting myself the work."

The owner also asked whether sys1 encourages the turn inflation. Measured
answer: no — every `routed` record shows one judgment per run, label
`single`, confidence 0.64; sys1 classifies once per run and never fans out
mid-run. The measured drivers are: the warmup skills are audits
(`project-warmup` v3.5.0's 15-item checklist and case-study prose,
`session-warmup`'s "read all in full"), the compiled-in prompt has no stop
for situate turns ("never offer a menu of next steps" collides with
warmup's required ask, and an open-ended task under "keep working until
complete" has no reachable end), and there is no effort ceiling for
trivial tasks (1.6 tool calls per turn; compaction never fired).

The session also exposed a defect: the owner's correction could not
persist — `remember` failed with "could not create /memories: Read-only
file system". Root cause: `bridge.rs` resolves the memory corpus root once
at face startup from the process cwd (`/` for the launched face), and a
run's corpus never follows the session's working folder.

## Acceptance criteria

- [x] C1: the compiled-in system prompt carries a situate carve-out (a
  warmup/orientation turn loads the few files that situate, reports
  compactly, and ends by asking what to work on; open plans and open
  ledger rows are state to report, never authorization) and an effort
  scaling rule (a simple or read-only task is a handful of tool calls and
  then the answer); prompt-pinning tests updated.
- [x] C2: `project-warmup` and `session-warmup` in `~/.agents/skills` are
  lean (~70 lines): one branch check, git state, the state surfaces
  (state doc, latest recap headline, plan index statuses, ledger open
  count), a compact report ending with the ask; the lessons kept as tight
  bullets; the explicit line that an active plan is context, not
  authorization.
- [x] C3: `docs/harness-orientation.md`'s Orient step becomes Situate
  (lean shape + the ruling + warmup is not the pipeline's first execution
  stage); `docs/architecture/skill-chains.md` and `CLAUDE.md`'s session
  protocol amended in one line each.
- [x] C4: repo facts `warmup-is-situate-and-align-never-execution.md` and
  `an-active-plan-is-context-not-authorization.md` carry the ruling into
  recall.
- [x] C5: a run's memory corpus root resolves from the session's working
  folder (falling back to the launch cwd); unit tests pin a folder-granted
  run resolving its corpus at the folder and remembering successfully.
- [x] C6: KI-533/534/535 filed; the gate green; commits on master, push;
  local deploy (rebuild → `~/.fleet/bin/fleet` → face relaunch); no
  release steps.

## Verification

The gate, the harness, one headless `fleet run "warmup" --yes` probe with
the new binary (bounded turns, no audits, the report ends with the ask),
the corpus-root unit tests, then the owner's live warmup re-run.

## Out of scope (named, not built)

A route-driven per-run turn budget; native situate-mode in the harness
loop; the lane_failed MCP spawn waste.
