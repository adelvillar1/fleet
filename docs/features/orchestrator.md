# The orchestrator

A conversation's **orchestrator** is the lead persona working it. The
persona's role carries a **workload**, its runs walk that workload's
chain, and the role **owns the threads it is assigned** — live, ended,
archived, disposed alike — so a role's work reads end-to-end long after
the spine's recency window has moved on.

One rule keeps the routing honest: the named workload is a **standing
default the judgment still runs under**. sys1 judges every task, and the
route row carries both words — `workload` (the judged label) and
`walked_as` (the chain that actually walked, present when the two differ).

## The four rulings (2026-11-13)

1. A persona carries a named workload; its runs route through that
   workload's chain.
2. Any conversation gets an orchestrator assignment. The orchestrator
   performs the router's duties for that conversation and can ask other
   bots to join, or spin one up for a role.
3. A role owns all the threads assigned to it, even when the dispatch
   used a thread that has since been disposed of — ownership is by
   assignment, permanent, lifecycle-independent.
4. Each role carries its own **work graph**: the spine's shape, scoped to
   what the role owns.

## What the orchestrator does

All of it is existing machinery, re-homed under the orchestrator identity:

- **delegates** to role bots — the `subagent` tool's `role` argument, and
  the child walks that role's workload through `route::prepare_child`;
- **convenes rooms** — the huddles surface's goal-plus-bots door, where
  the first bot named leads;
- **authoring personas** — the settings role writer and the face's
  persona door, including the workload picker;
- **hears its members** — the `message_orchestrator` tool, whose message
  lands in the orchestrator's transcript attributed to the sender.

## Where the facts live

| fact | record |
|---|---|
| the conversation's orchestrator | the log's `start` record (`personas`, first = lead) |
| the role's workload | the role file's `workload:` frontmatter, validated against the routing table's closed set at authoring |
| the walked chain | the route row's `walked_as`, beside the judged `workload` |
| the owned threads | `<home>/sessions-roles.json` (the ownership index), written at every dispatch |
| the role's work graph | read from that index plus each child's own `start` parent — never from the spine's budget-capped scans |

## Degradations, all named

- A role naming a workload the routing table does not carry is **refused
  at authoring** — a role file is not the place to invent a chain.
- A named workload whose chain does not resolve **falls back to the judged
  workload**, and the row's `reason` says so; the run never refuses
  mid-way for its persona's sake.
- A conversation with **no assignment** walks `hard`, byte-identical to
  every unassigned route before the orchestrator existed.

## Where it is visible

- `fleet roles` — each card's line names the workload (`the judged
  workload` when it carries none).
- Settings → roles — the role card shows its workload, its owned threads
  (archived flagged), and **its work graph**: the role at the root, its
  threads as nodes, and the delegation edges among them as nesting.
- The conversation head — a quiet `orchestrator · <name>` chip when the
  conversation has one; a room's lead bot carries the word on its chip.
- `/api/state` carries `orchestrator`, and `/api/settings`' roles document
  carries `workload`, `owned`, and the graph's `edges`.
