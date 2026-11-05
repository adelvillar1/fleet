---
name: registry-server-seeding-is-all-or-nothing
description: "project 2026-10-26 — FIXED d84d7d9: one unkeyed enabled MCP server used to refuse every headless run (the github incident); enabled_specs now resolves per entry and returns the specs beside the named absences, the router's own unkeyed-candidate pattern ported"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

`AgentOptions::seed_user_servers` → `mcp_registry::enabled_specs`
resolved EVERY enabled server's env up front and returned Err on the
first name that resolved to nothing, so `main.rs` exited with
`AgentError::Registry` before the run started. On this machine the
registry (`~/.fleet/mcp.json`) carried twelve enabled servers imported
from other harnesses, several with unresolvable names — so a headless
`fleet run` could not start at all. The 2026-10-25 incident: the MCP
scan had imported the servers with each source's own enabled flag,
`github` among them (Claude Desktop had it enabled) with
GITHUB_PERSONAL_ACCESS_TOKEN unset, so every headless run refused
while the browser face kept working. The only way past was dummy env
values in the process (`env GITHUB_PERSONAL_ACCESS_TOKEN=dummy … fleet
run …`), nothing written to the home.

**Instance fix (owner ruling "then disable it, why are you fighting
something that's clearly wrong"):** the four unkeyed enabled servers —
`github`, `node-code-sandbox-mcp`, `node_repl`, `openweather` — are
`"enabled": false` in the live registry, written directly to the JSON
because the tree carried a parallel session's broken in-flight KI-314
edit; verification ran on a HEAD worktree build and a by-hand run
completed end to end.

**Class fix — landed 2026-10-26 (`d84d7d9`, plan
2026-10-26-mcp-registry-named-absence.md completed, KI-319 filed).**
`enabled_specs` returns `EnabledRegistry { specs, absent }`: it
resolves per entry, an unkeyed server drops into `absent` with its
reason named, and only a registry file that cannot be read or parsed
still fails the call — the owner's config broken, not a missing key.
`seed_user_servers` hands the absences out and FOUR seeding sites print
one line each at startup in the skills-skipping voice (the CLI, the GUI
face, the TUI, and the cron catch-up — the TUI was a fifth site the
build discovered); the bridge's lane-call path resolves or falls
through to the vendored lanes without a registry refusal. The landed
shape differs from the design in one way: the return is a named struct
rather than the planned tuple, because clippy's complex-type rule
refused the tuple.

Four pins: the contract's own (an unkeyed entry is skipped WITH its
reason while its peers resolve; a disabled server names nothing at
all), a corrupt registry file still fails loud, the seeding returns Ok
with the absence named, and the incident's shape end to end —
`a_run_completes_over_a_registry_whose_server_is_unkeyed`: with the
four servers re-enabled and their env still unset, a run seeds,
completes, answers, and never declares their tools.

**How to apply:** a run that refuses before its first model call with
"server `X`: env `Y` resolves to nothing" was this shape; it is now
impossible — such a run seeds and names the absence. The pattern is
the router's own: an unkeyed candidate is a named skip and the walk
continues. Related: [[fleet-run-control-and-timeout-gaps]],
[[a-named-blocker-is-a-thing-to-remove]],
[[mcp-registry-live-import-state]].
