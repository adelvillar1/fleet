# The agent's behavioral assets are code — they carry git history

Direction note, 2026-11-13. The owner's ruling during the warmup-situate
work: the skills store should be git-tracked "like memories", and the same
extends to tools, workflows, and loops.

## The principle

Anything that shapes what the agent does persistently — a skill's
procedure, a tool's surface, a workflow's steps, a loop's schedule — is
code the agent runs, and code the owner rules on. A ruling that lands as
an in-place overwrite has no history: nobody can see what changed, when,
or why (the warmup lean-rewrite of 2026-11-13 was exactly that until the
store's git caught it). Each behavioral store gets the memories
corpus's treatment:

- **tracked** — a git repo with history;
- **authored as commits** — a change is a commit with an honest message
  (rulings name their date and their source);
- **runtime churn as bookkeeping** — the machine's own edits (recall
  bumps, lifecycle stamps, index refreshes) are their own `chore`-class
  commits, never mixed with authored change;
- **a remote** — the store backs up and syncs like the code it shapes.

## The mapping (what exists today)

| Asset | Store | State |
|---|---|---|
| memories (facts) | `memories/` in each repo; the fleet home's own corpus | tracked in-repo; churn committed by the daily cycle |
| skills | `~/.agents/skills` (the cross-harness canonical store; `~/.zcode/skills` symlinks into it) | its own repo + remote (`skills-meta-20260830`), drifted state reconciled and pushed 2026-11-13; fleet's skill proposals (`~/.fleet/skills/proposals`) stage drafts, and a promotion into the store becomes a commit |
| tools | the MCP registry (`mcp.json`), the tool set, the settings writers | not yet tracked — track when a change to them stops being obvious |
| workflows / loops | the workflow and loop surfaces as they land (today's loop store is the cron job store — runtime state, not authored content; the authored definitions are what get tracked) | direction, not built |

## The line this draws

Runtime state (session logs, counters, cache) is data, not code — it
stays out of version control. Authored behavior (skills, tools, workflow
and loop definitions) is code, and gets the code's discipline. When a new
behavioral surface is built, its store ships with its repo — tracking
retrofitted is how the drift happened.
