# Plans

Plans are contracts. One file per body of work, named `YYYY-MM-DD-<slug>.md`, with YAML frontmatter:

```yaml
---
status: draft|active|completed|cancelled
created: YYYY-MM-DD
updated: YYYY-MM-DD
slug: feature-name
---
```

Sections: Context, Approach, Acceptance Criteria (verifiable, specific), Files to be touched, Out of scope, Verification, Linked artifacts.

Rules:

- Draft a plan before non-trivial work (≥15 min or >2 files). Trivial work may go direct.
- The plan file is the source of truth for "is X done" — not the recap, not memory.
- Update `status` in the same turn the work lands. `active → completed` when every acceptance criterion is confirmed met; `active → cancelled` when superseded or abandoned. Never reopen a completed plan — start a new one.
- Mid-implementation scope changes: update the plan's acceptance criteria first, then build.
- A plan's evidence homes are `docs/` artifacts and live measurements — reviewed, versioned,
  and carried by git history. A memory fact is a pointer at best: cite it beside a short inline
  quote of what it said, so an archived, consolidated, or superseded fact degrades the citation
  to a paraphrase instead of a hole (KI-323).
- Before moving a plan to `completed`, run `python3 scripts/plan_check.py`: it lists every
  completed plan whose acceptance criteria still carry an open box, and exits 1 naming them
  (KI-398). `.githooks/pre-commit` runs the same check whenever a plan file is staged, so the
  status line and the boxes cannot disagree in a commit — enable it once per clone with
  `git config core.hooksPath .githooks`.

## The execution declaration

An `active` plan may carry a fenced `graph` block: the machine-readable
contract beside the human one. Only an approved (`active`) plan generates a
graph — `draft` and `completed` generate none, which is why the board shows a
plan's tasks exactly while the plan is being executed.

````text
```graph
{
  "nodes": [
    {
      "id": "read-the-thing",
      "cost": 2,
      "approach": "what this node does, in one line",
      "serves": "C1",
      "assignee": "scholar",
      "prereqs": [],
      "paths": ["src/thing.rs"],
      "capabilities": { "tools": ["bash"], "skills": [], "workflows": [], "loops": [] }
    }
  ]
}
```
````

Each node is one **atomic delegatable task**. `prereqs` names the nodes that
must verify first; `serves` is the acceptance criterion it answers; `paths` are
the files it admits (two nodes naming one path never run concurrently).

`assignee` is optional and names the role or persona that works the node —
what the board's card shows as "who it is for". A node that names nobody
inherits the plan's own workload (its frontmatter `execution:` verdict:
`single`, `mixture` or `swarm`). A plan that says nothing about assignment
produces honestly unassigned cards rather than an invented owner.

The declaration is the contract for the harness; the prose above it stays the
contract for humans. Update the acceptance criteria first, then the
declaration — a plan edited after its graph was generated carries a digest
mismatch, which is named with both versions rather than silently mismapped.

Inherited records (copied byte-identical from the DeepSeek Harness fork when Fleet became its own repository):

- `2026-10-12-fleet-native-app.md` — slice 1 design record; still `active` (C3 waits on account balance; slices 2+ unplanned).
- `2026-10-12-fleet-dashboards.md` — the router-board and second-brain-board plugins in the fork's fleet profile; `completed`.
