---
name: fleet-mcp-server-registry-plan-unbuilt
description: "2026-10-23 — the MCP server registry plan (docs/plans/2026-10-22-mcp-server-registry.md) was verified never-built, amended (dea0d1a), then implemented AS DESIGNED the same day: five gated commits a063f66/82b7679/32a1099/14e3bfe/4b7a9ec + close-out 464acc0, status completed, 10/10 criteria ticked, plan-gate PASS. Registry = ~/.fleet/mcp.json (env NAMES only), four-harness scan + import, run pre-join, health probe, settings card. Two build amendments recorded in the plan; live scan leaked a GitHub PAT once be…"
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

The full arc of `docs/plans/2026-10-22-mcp-server-registry.md`, in one day
(2026-10-23): owner asked to verify its status → I found it **draft, never
built** (0 of 9 criteria, every promised artifact absent; the drafting session
hit the 25-turn cap five runs in a row and the owner's `/goal implement as
designed` had gone to the chat-UX plan instead) → owner said "amend anything
out of place and let me review" → I corrected three wrong claims (`dea0d1a`:
"five lanes" was six; "eleven of sixteen stdio" was thirteen; and increment 1's
"reuse set_lane's secret guard" was inverted — `set_lane` refuses
credential-shaped names, which is exactly what a registry must store) → the
owner's next turn was `/goal implement the mcp plan as designed`, and all five
increments landed.

**Shipped state (pushed, gate green, 1391 passed / 1 ignored, 148 face checks):**
- `a063f66` — `src/mcp_registry.rs` (mcp.json, validation, reserved names =
  the six lanes + four built-ins, the no-value schema), writers in
  `settings.rs` (`set_server`/`remove_server`/`set_server_enabled`/
  `record_server_probe`), `fleet mcp` list/enable/disable/remove with cards.
- `82b7679` — scan over the four foreign configs (Claude Desktop/Claude
  Code/Cursor/Codex; Codex TOML read by a purpose-built subset parser), import
  (`harness:name` disambiguation), advisory badges derived from the launch
  shape, `-e NAME=VALUE` extraction at the read.
- `32a1099` — run wiring: `AgentOptions::seed_user_servers` on the two
  owner-facing sites (headless run/chat + bridge /api/run; NOT subagents, NOT
  cron), `LaneRoster::pre_join` before the first request, `AgentError::Registry`
  for an unresolved env name, `lane_tools` records naming both ends.
- `14e3bfe` — `fleet mcp health`: spawn/handshake/list/report/teardown per
  server, outcome into `last_probe`, exit = fleet's health. `fleet mcp` joined
  the async command set (`needs_runtime` in main.rs).
- `4b7a9ec` — the settings card (`#/settings`, beneath lanes): probe dots,
  enable/probe/remove, the scan's discovery rows with badges and import,
  unsupported rows shown with a disabled import, configured ones collapsed.
  `server-scan`/`server-probe` pre-empt `spawn_blocking` in `settings_write`
  (they need the runtime); the rest ride `settings_apply`.
- `464acc0` — `docs/features/mcp.md` (new), pointer index, TECHNICAL contract
  rows, FUNCTIONAL settings + run-behavior sentences, KI-284 note, plan
  ticked/completed.

**The two build amendments (in the plan, re-gated PASS):**
1. **D3's args half, discovered by leaking.** The live scan's first render
   printed a real GitHub PAT from the owner's Claude Desktop config
   (`docker run -e GITHUB_PERSONAL_ACCESS_TOKEN=ghp_…`). The fix: the scan
   extracts `-e NAME=VALUE` pairs AT THE READ — the row's args become
   `-e NAME`, the NAME joins `env_names`, no field of the row carries the
   value — plus display redaction (`NAME=<resolves at spawn>`, URL query keys
   `apiKey=` etc → `<redacted>`). Extract at scan time, not import time, so
   nothing downstream can repeat what it never received. **The PAT crossed a
   session log; the owner was told to consider rotating it.** Named residual:
   a credential inside a server's own URL cannot be extracted; the registry
   stores it verbatim, no card prints it.
2. **D6's mechanism.** The four badges are read off the entry's own launch
   shape (`docker`→containerized, `npx/uvx`→pinned package, `python3/node`→
   bare interpreter, `url`→remote endpoint), not asked of the sys1 lane — the
   label is a fact the config declares, and an ask would spend the judgment
   budget per row on a fact in hand. Amending a plan's mechanism in the same
   commit as the increment is the repo's own rule, applied.

**Verification habits that paid:** grep the served asset and the imported file
for the credential strings you know exist on the machine — one grep found the
leak the marker test's env-block fixture could not. The harness's selector
stub takes no `:not()`; filter by className. `CARGO_BIN_EXE_*` exists only in
integration tests, not lib unit tests. A pre-join that joins every injected
spec shifts every injected-spec test's record indexes and event order — two
tests were updated to the new shape, deliberately, because production seeds
only registry servers.

The docs-gate reported two judgment gaps (FUNCTIONAL 0.458, KI-284 0.21) that
did not move across re-runs even with changed text; the artifacts exist and
name the plan — recorded in the close-out commit rather than distorting prose
to move a model's threshold.

**The lesson that survives from the unbuilt phase:** a zero-implementation
draft should stay `draft`; tell never-built from stale-draft by artifact
counting, not checkboxes — `git log -- <plan>`, then grep for EVERY promised
file and symbol (a hit only inside the plan file is no hit). And when amending
before a build, verify every factual claim against the tree: this plan was
wrong in three places, each of which would have misled the build.

Related: [[mcp-manager-build-state-and-coordination]] (the tool-surface plan's
ownership boundaries around this code), [[plan-is-a-file-not-prose]],
[[a-flag-that-conflates-a-tree-fact-with-a-machine-fact]] (the gate failure
this session also fixed).
