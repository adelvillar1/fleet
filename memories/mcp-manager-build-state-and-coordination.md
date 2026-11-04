---
name: mcp-manager-build-state-and-coordination
description: "2026-10-23 — the MCP server registry plan is COMPLETE and pushed (increments 1-5: a063f66, 82b7679, 32a1099, 14e3bfe, 4b7a9ec + close-out 464acc0; plan status completed, 10/10 ticked); the tool-surface plan's ownership boundaries stand (manager owns mcp_registry/mcp.json/fleet mcp/the settings card; tool plan reads enabled_specs, owns declarations ordering); a delivered contract ships as written (no retro-editing C2)"
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

Recorded 2026-10-23 while planning the tool surface. The earlier memory
[[fleet-mcp-server-registry-plan-unbuilt]] says the plan was never built —
**that is now stale for increments 1–3.** That file's own discrimination rule
is the one that caught it: count artifacts and feature commits, do not count
checkboxes.

**Measured state** (from `git log` and the tree during that session):

| Increment | State | Evidence |
|---|---|---|
| 1 — registry, validation, writers, `fleet mcp` list/enable/disable/remove | landed | `a063f66`; `src/mcp_registry.rs:98-326` |
| 2 — discovery and import, names and shapes only | landed | `82b7679`; scan/import at `src/mcp_registry.rs:528-1103` |
| 3 — run wiring, enabled servers pre-join at run start | landed | `32a1099`; `enabled_specs` at `src/mcp_registry.rs:156` |
| 4 — health probe | **uncommitted in the working tree** | the `Health` verb in `src/cli.rs`, `mcp_health_command` in `src/main.rs`, plus `src/mcp_registry.rs` and `tests/integration.rs` dirty |
| 5 — settings board card | not started | — |

The plan file itself is therefore a stale ledger too: it still reads
`status: draft` with 0 of 10 criteria ticked while half its increments are
committed. Reading the plan's frontmatter to judge progress is exactly the
mistake its own "how to tell never-built from stale-draft" section warns about,
inverted.

**The two rulings the tool-surface plan made so the two plans can coexist**
(`docs/plans/2026-10-23-tool-surface-and-lane-resolution.md`, D12):

1. **Ownership, not negotiation.** The manager owns `src/mcp_registry.rs`,
   `mcp.json`, the `fleet mcp` verbs, and the settings servers card; the tool
   plan reads `enabled_specs` and `read_servers` and never edits those. The
   tool plan owns `declarations()` ordering and tiers, `src/tools.rs`, the new
   `tool_*` modules, `src/mcp.rs` protocol, `src/compaction.rs` and the session
   record. Both touch `src/gui/assets/app.js` but in different functions
   (`paintSettings` vs the tool-card renderer).
2. **A discovered-but-unconfigured server is a NAME and nothing else.** It
   may appear as a row on the manager's card; it never enters the tool
   catalog and can never be dispatched. This is the boundary between "the
   manager exposes what the machine has" and "the tool surface makes only
   enabled things callable."

**The one real conflict, and how it was settled.** The manager's C2 says an
enabled server's tools are "declared from the first request"; the tool plan
tiers any catalog past 28 tools or 8k definition tokens behind `tool_search`.
I first wrote that the manager's criterion "needs a one-line amendment". The
revision that stands: **a delivered contract ships as written.** Below the
threshold both hold; above it reachability replaces declaration, and the
supersession is recorded in `docs/features/mcp.md` when the tool plan's
increment 4 lands, with the owner deciding then whether the completed plan's
C2 is amended. Retro-editing a delivered criterion to fit a later plan is how
a ledger stops being trustworthy.

**Sequencing (now unblocked).** The manager is done, so the tool plan's
face-work restriction is lifted — but the ownership boundaries still stand: the
tool plan reads `enabled_specs` and `read_servers` and never edits
`mcp_registry.rs`, `mcp.json`, or the settings servers card; `declarations()`
ordering, `src/tools.rs`, the `tool_*` modules and the session record are its
side.

**Closed out the same day.** Increment 4 landed as `14e3bfe` and increment 5
as `4b7a9ec`, with the plan flipped to `completed` in `464acc0` — all five
increments committed. The tool plan therefore started increments 1–3
immediately (which touch none of the manager's files) and its face work is no
longer blocked. The tiering landed in `05915c0` and is now the standing
supersession of C2 above the threshold.

**How to apply:** before planning anything that touches tools or MCP, run the
artifact check on `mcp_registry.rs` and `git log --oneline -- src/mcp_registry.rs`
first — the manager is moving, and an uncommitted increment will not appear in
history (this file's own table was written from a tree where increments 4–5
were still uncommitted, and both landed hours later). Anything you add around enabled servers must consume `enabled_specs`,
not re-derive which servers are on. Related:
[[fleet-tool-surface-analysis-2026-10-23]],
[[shared-file-commits-can-sweep-another-sessions-work]].
